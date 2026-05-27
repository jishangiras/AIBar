<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-shell';
  import { api } from '$lib/api/tauri';
  import { servicesStore } from '$lib/stores/services.svelte';
  import ServiceCard from '$lib/components/ServiceCard.svelte';
  import DetailPanel from '$lib/components/DetailPanel.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import type { ServiceStatus } from '$lib/types';

  // ── View state ──────────────────────────────────────────────────────
  type View = 'main' | 'settings';
  let view = $state<View>('main');

  // ── Settings state ───────────────────────────────────────────────────
  const SERVICES = [
    // ── Web session services (plan usage / message caps) ─────────────────
    { id: 'claude_web',  name: 'Claude.ai',  icon: '✦', org: 'Anthropic',     keyHint: 'sk-ant-sid…', keyPage: 'https://claude.ai/settings/limits',             note: 'Session cookie from claude.ai — monitors Pro plan usage, credits & routine runs. Get from DevTools → Application → Cookies → sessionKey' },
    { id: 'chatgpt_web', name: 'ChatGPT',    icon: '⊕', org: 'OpenAI',        keyHint: 'eyJhbGci…',   keyPage: 'https://chatgpt.com/',                          note: 'Session cookie from chatgpt.com — monitors message caps (5-hour & weekly). Get from DevTools → Application → Cookies → __Secure-next-auth.session-token' },
    // ── API key services (rate-limit headers) ────────────────────────────
    { id: 'claude',      name: 'Claude API', icon: '✦', org: 'Anthropic',     keyHint: 'sk-ant-api…', keyPage: 'https://console.anthropic.com/settings/keys',   note: 'Paid API key — monitors per-minute request & token rate limits' },
    { id: 'openai',      name: 'OpenAI API', icon: '⊕', org: 'OpenAI',        keyHint: 'sk-…',        keyPage: 'https://platform.openai.com/api-keys',           note: 'Paid API key — monitors per-minute request & token rate limits' },
    { id: 'gemini',      name: 'Gemini',     icon: '◈', org: 'Google',        keyHint: 'AIza…',       keyPage: 'https://aistudio.google.com/app/apikey',          note: 'Free tier available' },
    { id: 'grok',        name: 'Grok',       icon: '✕', org: 'xAI',           keyHint: 'xai-…',       keyPage: 'https://console.x.ai/',                          note: 'Paid API key' },
    { id: 'perplexity',  name: 'Perplexity', icon: '⊗', org: 'Perplexity AI', keyHint: 'pplx-…',     keyPage: 'https://www.perplexity.ai/settings/api',         note: 'Paid API key' },
  ] as const;

  let connected = $state<Set<string>>(new Set());
  let expanded  = $state<string | null>(null);
  let keyInput  = $state<Record<string, string>>({});
  let saving    = $state<Record<string, boolean>>({});
  let flash     = $state<Record<string, { ok: boolean; msg: string } | null>>({});

  // ── Main view state ──────────────────────────────────────────────────
  let refreshingId  = $state<string | null>(null);
  let refreshingAll = $state(false);
  let globalError   = $state<string | null>(null);

  const aggregateStatus = $derived.by<ServiceStatus>(() => {
    const svcs = servicesStore.services;
    if (!svcs.length) return 'unknown';
    const order: ServiceStatus[] = ['unknown', 'red', 'yellow', 'green'];
    return svcs.reduce<ServiceStatus>(
      (worst, s) => order.indexOf(s.status) < order.indexOf(worst) ? s.status : worst,
      'green'
    );
  });

  // ── Actions ──────────────────────────────────────────────────────────
  async function handleRefresh(id?: string) {
    try {
      if (id) { refreshingId = id; await servicesStore.refresh(id); }
      else     { refreshingAll = true; await servicesStore.refresh(); }
    } catch (e) {
      showError(String(e));
    } finally {
      refreshingId = null;
      refreshingAll = false;
    }
  }

  function showError(msg: string) {
    globalError = msg;
    setTimeout(() => { globalError = null; }, 4000);
  }

  function drag(e: MouseEvent) {
    if (!(e.target as HTMLElement).closest('button')) getCurrentWindow().startDragging();
  }

  // ── Settings actions ─────────────────────────────────────────────────
  async function loadConnected() {
    try { connected = new Set(await api.getStoredServiceIds()); } catch {}
  }

  function toggleExpand(id: string) {
    expanded = expanded === id ? null : id;
    if (expanded === id) keyInput = { ...keyInput, [id]: '' };
  }

  async function saveKey(id: string) {
    const key = keyInput[id]?.trim();
    if (!key) return;
    saving = { ...saving, [id]: true };
    flash  = { ...flash, [id]: null };
    try {
      await api.saveApiKey(id, key);
      connected = new Set([...connected, id]);
      keyInput  = { ...keyInput, [id]: '' };
      expanded  = null;
      flash = { ...flash, [id]: { ok: true, msg: 'Saved!' } };
      // Kick off a background fetch so the service card appears
      invoke('refresh_service', { serviceId: id }).catch(() => {});
    } catch (e) {
      flash = { ...flash, [id]: { ok: false, msg: String(e) } };
    } finally {
      saving = { ...saving, [id]: false };
      setTimeout(() => { flash = { ...flash, [id]: null }; }, 4000);
    }
  }

  async function disconnectKey(id: string) {
    try {
      await api.deleteApiKey(id);
      connected = new Set([...connected].filter(x => x !== id));
      expanded = null;
    } catch (e) {
      flash = { ...flash, [id]: { ok: false, msg: String(e) } };
    }
  }

  function goSettings() { view = 'settings'; loadConnected(); }
  function goMain()     { view = 'main'; servicesStore.load(); }

  // ── Lifecycle ────────────────────────────────────────────────────────
  onMount(async () => {
    await servicesStore.load();
    await servicesStore.startListening();

    // Navigate event from tray menu or Rust command
    await listen<string>('navigate', (e) => {
      if (e.payload === 'settings') goSettings();
    });

    // Reload when window becomes visible
    const onVisible = () => {
      if (document.visibilityState === 'visible') servicesStore.load();
    };
    document.addEventListener('visibilitychange', onVisible);
    return () => document.removeEventListener('visibilitychange', onVisible);
  });
</script>

<!-- Root container -->
<div
  class="relative w-[420px] h-[520px] flex flex-col overflow-hidden rounded-xl"
  style="background: var(--bg); border: 1px solid var(--border); box-shadow: var(--shadow);"
>

  <!-- ── Title bar ─────────────────────────────────────────────────── -->
  <div
    class="flex items-center justify-between px-4 py-3 shrink-0 cursor-grab active:cursor-grabbing"
    style="border-bottom: 1px solid var(--border);"
    onmousedown={drag}
  >
    {#if view === 'main'}
      <div class="flex items-center gap-2.5">
        <StatusDot status={aggregateStatus} size="md" pulse={aggregateStatus === 'red'} />
        <span class="text-sm font-semibold tracking-tight" style="color: var(--fg);">AIBar</span>
      </div>
      <div class="flex items-center gap-0.5">
        <button class="p-1.5 rounded-lg text-sm transition-colors disabled:opacity-30"
                style="color: var(--fg3);"
                onmouseenter={e => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
                onmouseleave={e => (e.currentTarget as HTMLElement).style.background = 'transparent'}
                onclick={() => handleRefresh()} disabled={refreshingAll} title="Refresh all">
          {refreshingAll ? '⏳' : '↻'}
        </button>
        <button class="p-1.5 rounded-lg text-sm transition-colors"
                style="color: var(--fg3);"
                onmouseenter={e => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
                onmouseleave={e => (e.currentTarget as HTMLElement).style.background = 'transparent'}
                onclick={goSettings} title="Settings">⚙</button>
        <button class="p-1.5 rounded-lg text-sm transition-colors"
                style="color: var(--fg3);"
                onmouseenter={e => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
                onmouseleave={e => (e.currentTarget as HTMLElement).style.background = 'transparent'}
                onclick={() => getCurrentWindow().hide()} title="Close">✕</button>
      </div>
    {:else}
      <div class="flex items-center gap-2.5">
        <button class="p-1.5 rounded-lg text-sm transition-colors"
                style="color: var(--fg3);"
                onmouseenter={e => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
                onmouseleave={e => (e.currentTarget as HTMLElement).style.background = 'transparent'}
                onclick={goMain} title="Back">←</button>
        <span class="text-sm font-semibold tracking-tight" style="color: var(--fg);">API Keys</span>
      </div>
      <button class="text-xs px-3 py-1.5 rounded-lg transition-colors"
              style="color: var(--fg3);"
              onmouseenter={e => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
              onmouseleave={e => (e.currentTarget as HTMLElement).style.background = 'transparent'}
              onclick={goMain}>Done</button>
    {/if}
  </div>

  <!-- ── Error banner ──────────────────────────────────────────────── -->
  {#if globalError}
    <div class="px-4 py-2 text-xs text-red-500 text-center shrink-0"
         style="background: rgba(239,68,68,0.08); border-bottom: 1px solid rgba(239,68,68,0.2);">
      {globalError}
    </div>
  {/if}

  <!-- ════════════════════════════════════════════════════════════════ -->
  <!-- MAIN VIEW -->
  <!-- ════════════════════════════════════════════════════════════════ -->
  {#if view === 'main'}
    <div class="flex-1 p-4 overflow-y-auto">
      {#if servicesStore.loading}
        <div class="h-full flex items-center justify-center">
          <div class="flex flex-col items-center gap-3">
            <div class="w-5 h-5 border-2 rounded-full animate-spin"
                 style="border-color: var(--border); border-top-color: var(--fg3);"></div>
            <span class="text-xs" style="color: var(--fg3);">Loading…</span>
          </div>
        </div>
      {:else if servicesStore.services.length === 0}
        <div class="h-full flex items-center justify-center">
          <div class="flex flex-col items-center gap-3 text-center px-6">
            <span class="text-3xl opacity-50">🔑</span>
            <p class="text-sm leading-relaxed" style="color: var(--fg3);">
              No services connected yet.
            </p>
            <button class="text-sm px-4 py-2 rounded-lg font-medium transition-colors"
                    style="background: var(--btn); color: var(--fg2);"
                    onmouseenter={e => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
                    onmouseleave={e => (e.currentTarget as HTMLElement).style.background = 'var(--btn)'}
                    onclick={goSettings}>
              Add API Keys →
            </button>
          </div>
        </div>
      {:else}
        <div class="grid grid-cols-3 gap-2.5">
          {#each servicesStore.services as service (service.id)}
            <ServiceCard {service} onclick={() => servicesStore.select(service.id)} />
          {/each}
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="px-4 py-2 shrink-0 flex items-center justify-between"
         style="border-top: 1px solid var(--border);">
      <span class="text-[10px]" style="color: var(--fg3);">
        {servicesStore.services.length} service{servicesStore.services.length !== 1 ? 's' : ''}
      </span>
      <span class="text-[10px]" style="color: var(--fg3);">Refreshes every 10 min</span>
    </div>

    <!-- Detail overlay -->
    {#if servicesStore.selected}
      <DetailPanel
        service={servicesStore.selected}
        onclose={() => servicesStore.select(null)}
        onrefresh={() => handleRefresh(servicesStore.selectedId!)}
        refreshing={refreshingId === servicesStore.selectedId}
      />
    {/if}

  <!-- ════════════════════════════════════════════════════════════════ -->
  <!-- SETTINGS VIEW -->
  <!-- ════════════════════════════════════════════════════════════════ -->
  {:else}
    <div class="flex-1 overflow-y-auto">
      <!-- Note banner -->
      <div class="mx-4 mt-3 mb-2 px-3 py-2.5 rounded-lg text-[11px] leading-relaxed"
           style="background: var(--bg2); color: var(--fg3); border: 1px solid var(--border);">
        These are <strong style="color: var(--fg2);">API keys</strong> — separate from chat subscriptions.
        Stored in your OS keychain only.
      </div>

      <!-- Service rows -->
      <div class="px-4 pb-4 space-y-2">
        {#each SERVICES as svc}
          {@const isConnected = connected.has(svc.id)}
          {@const isExpanded  = expanded === svc.id}
          {@const f = flash[svc.id]}

          <div class="rounded-xl overflow-hidden"
               style="border: 1px solid {isConnected ? 'rgba(34,197,94,0.3)' : 'var(--border)'}; background: var(--bg2);">

            <!-- Row -->
            <div class="flex items-center justify-between px-3 py-2.5 gap-2">
              <div class="flex items-center gap-2.5 min-w-0">
                <span class="text-base shrink-0">{svc.icon}</span>
                <div class="min-w-0">
                  <p class="text-sm font-medium leading-none truncate" style="color: var(--fg);">{svc.name}</p>
                  <p class="text-[10px] mt-0.5" style="color: var(--fg3);">{svc.org}</p>
                </div>
              </div>
              <div class="flex items-center gap-2 shrink-0">
                {#if f}
                  <span class="text-[11px] font-medium"
                        style="color: {f.ok ? '#22c55e' : '#ef4444'};">{f.msg}</span>
                {:else if isConnected && !isExpanded}
                  <span class="w-1.5 h-1.5 rounded-full bg-[#22c55e]"></span>
                {/if}
                <button class="text-xs px-2.5 py-1 rounded-lg transition-colors font-medium"
                        style="background: var(--btn); color: var(--fg2);"
                        onmouseenter={e => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
                        onmouseleave={e => (e.currentTarget as HTMLElement).style.background = 'var(--btn)'}
                        onclick={() => toggleExpand(svc.id)}>
                  {isExpanded ? 'Cancel' : isConnected ? 'Change' : 'Connect'}
                </button>
              </div>
            </div>

            <!-- Expanded connect flow -->
            {#if isExpanded}
              <div class="px-3 pb-3 space-y-2.5" style="border-top: 1px solid var(--border); padding-top: 10px;">
                <p class="text-[11px]" style="color: var(--fg3);">{svc.note}</p>

                <button class="w-full text-left text-xs px-3 py-2 rounded-lg transition-colors font-medium"
                        style="background: var(--btn); color: var(--fg2);"
                        onmouseenter={e => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
                        onmouseleave={e => (e.currentTarget as HTMLElement).style.background = 'var(--btn)'}
                        onclick={() => open(svc.keyPage)}>
                  Open {svc.org} API Keys ↗
                </button>

                <div class="flex gap-2">
                  <input
                    type="password"
                    class="flex-1 rounded-lg px-3 py-1.5 text-xs font-mono outline-none"
                    style="background: var(--bg); border: 1px solid var(--border); color: var(--fg);"
                    onfocus={e  => (e.currentTarget as HTMLElement).style.borderColor = 'var(--fg3)'}
                    onblur={e   => (e.currentTarget as HTMLElement).style.borderColor = 'var(--border)'}
                    placeholder={svc.keyHint}
                    bind:value={keyInput[svc.id]}
                    onkeydown={e => e.key === 'Enter' && saveKey(svc.id)}
                    autofocus
                  />
                  <button class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors disabled:opacity-40"
                          style="background: rgba(34,197,94,0.15); color: #22c55e;"
                          onclick={() => saveKey(svc.id)}
                          disabled={saving[svc.id] || !keyInput[svc.id]?.trim()}>
                    {saving[svc.id] ? '…' : 'Save'}
                  </button>
                </div>

                {#if isConnected}
                  <button class="text-[11px] transition-colors"
                          style="color: var(--fg3);"
                          onmouseenter={e => (e.currentTarget as HTMLElement).style.color = '#ef4444'}
                          onmouseleave={e => (e.currentTarget as HTMLElement).style.color = 'var(--fg3)'}
                          onclick={() => disconnectKey(svc.id)}>
                    Disconnect →
                  </button>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>

      <!-- Connected count -->
      <p class="text-center text-[10px] pb-4" style="color: var(--fg3);">
        {connected.size} of {SERVICES.length} connected
      </p>
    </div>
  {/if}
</div>
