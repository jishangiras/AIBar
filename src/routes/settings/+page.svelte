<script lang="ts">
  import { onMount } from 'svelte';
  import { open } from '@tauri-apps/plugin-shell';
  import { invoke } from '@tauri-apps/api/core';
  import { api } from '$lib/api/tauri';

  const SERVICES = [
    {
      id: 'claude',
      name: 'Claude',
      icon: '✦',
      org: 'Anthropic',
      keyHint: 'sk-ant-api…',
      keyPage: 'https://console.anthropic.com/settings/keys',
      note: 'Requires paid API access (separate from Claude.ai Pro)',
    },
    {
      id: 'openai',
      name: 'ChatGPT',
      icon: '⊕',
      org: 'OpenAI',
      keyHint: 'sk-…',
      keyPage: 'https://platform.openai.com/api-keys',
      note: 'Requires paid API access (separate from ChatGPT Plus)',
    },
    {
      id: 'gemini',
      name: 'Gemini',
      icon: '◈',
      org: 'Google',
      keyHint: 'AIza…',
      keyPage: 'https://aistudio.google.com/app/apikey',
      note: 'Free tier available via Google AI Studio',
    },
    {
      id: 'grok',
      name: 'Grok',
      icon: '✕',
      org: 'xAI',
      keyHint: 'xai-…',
      keyPage: 'https://console.x.ai/',
      note: 'Requires xAI API access',
    },
    {
      id: 'perplexity',
      name: 'Perplexity',
      icon: '⊗',
      org: 'Perplexity AI',
      keyHint: 'pplx-…',
      keyPage: 'https://www.perplexity.ai/settings/api',
      note: 'Requires paid API access',
    },
  ] as const;

  let connected = $state<Set<string>>(new Set());
  let expanded = $state<string | null>(null);
  let keyInput = $state<Record<string, string>>({});
  let saving = $state<Record<string, boolean>>({});
  let flash = $state<Record<string, { type: 'ok' | 'err'; msg: string } | null>>({});

  onMount(async () => {
    try {
      const ids = await api.getStoredServiceIds();
      connected = new Set(ids);
    } catch (e) {
      console.error('Failed to load stored service IDs:', e);
    }
  });

  function toggleExpand(id: string) {
    expanded = expanded === id ? null : id;
    if (expanded === id) keyInput = { ...keyInput, [id]: '' };
  }

  async function saveKey(id: string) {
    const key = keyInput[id]?.trim();
    if (!key) return;

    saving = { ...saving, [id]: true };
    flash = { ...flash, [id]: null };

    try {
      await api.saveApiKey(id, key);
      connected = new Set([...connected, id]);
      keyInput = { ...keyInput, [id]: '' };
      expanded = null;
      flash = { ...flash, [id]: { type: 'ok', msg: 'Connected!' } };

      // Kick off refresh — shows the service in the main bar
      // Don't await; let it run in background and show result via event
      invoke('refresh_service', { serviceId: id }).catch((e) => {
        console.error('refresh_service failed:', e);
      });
    } catch (e) {
      flash = { ...flash, [id]: { type: 'err', msg: `Failed to save: ${e}` } };
    } finally {
      saving = { ...saving, [id]: false };
      setTimeout(() => { flash = { ...flash, [id]: null }; }, 4000);
    }
  }

  async function disconnect(id: string) {
    try {
      await api.deleteApiKey(id);
      connected = new Set([...connected].filter((x) => x !== id));
      expanded = null;
    } catch (e) {
      flash = { ...flash, [id]: { type: 'err', msg: `Failed: ${e}` } };
    }
  }
</script>

<div class="min-h-screen flex flex-col select-none" style="background: var(--bg); color: var(--fg);">
  <!-- Header -->
  <div class="flex items-center justify-between px-5 py-3.5" style="border-bottom: 1px solid var(--border);">
    <h1 class="text-sm font-semibold">API Connections</h1>
    <button
      class="text-xs px-3 py-1.5 rounded-lg transition-colors"
      style="color: var(--fg3);"
      onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
      onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'transparent'}
      onclick={() => invoke('close_settings')}
    >Done</button>
  </div>

  <!-- Info banner -->
  <div class="mx-5 mt-4 mb-2 p-3 rounded-lg text-xs leading-relaxed"
       style="background: var(--bg2); color: var(--fg3); border: 1px solid var(--border);">
    <strong style="color: var(--fg2);">Note:</strong> This app requires <em>API keys</em>, which are
    separate from chat subscriptions (Claude Pro, ChatGPT Plus, etc.).
    Keys are stored in your OS keychain and never sent anywhere.
  </div>

  <!-- Service list -->
  <div class="flex-1 overflow-y-auto px-5 pb-5 pt-2 space-y-2">
    {#each SERVICES as svc}
      {@const isConnected = connected.has(svc.id)}
      {@const isExpanded = expanded === svc.id}
      {@const f = flash[svc.id]}

      <div
        class="rounded-xl overflow-hidden transition-colors"
        style="border: 1px solid {isConnected ? 'rgba(34,197,94,0.25)' : 'var(--border)'}; background: var(--bg2);"
      >
        <!-- Card row -->
        <div class="flex items-center justify-between px-4 py-3 gap-3">
          <div class="flex items-center gap-2.5 min-w-0">
            <span class="text-base shrink-0">{svc.icon}</span>
            <div class="min-w-0">
              <p class="text-sm font-medium leading-none" style="color: var(--fg);">{svc.name}</p>
              <p class="text-[10px] mt-0.5 truncate" style="color: var(--fg3);">{svc.org}</p>
            </div>
          </div>

          <div class="flex items-center gap-2 shrink-0">
            {#if f}
              <span class="text-[11px] font-medium"
                    style="color: {f.type === 'ok' ? '#22c55e' : '#ef4444'};">
                {f.msg}
              </span>
            {:else if isConnected}
              <div class="flex items-center gap-1.5">
                <span class="w-1.5 h-1.5 rounded-full bg-[#22c55e]"></span>
                <span class="text-[11px] text-[#22c55e]">Connected</span>
              </div>
            {/if}

            <button
              class="text-xs px-3 py-1.5 rounded-lg transition-colors font-medium"
              style="background: var(--btn); color: var(--fg2);"
              onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
              onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'var(--btn)'}
              onclick={() => toggleExpand(svc.id)}
            >
              {isExpanded ? 'Cancel' : isConnected ? 'Change' : 'Connect'}
            </button>
          </div>
        </div>

        <!-- Expandable connect flow -->
        {#if isExpanded}
          <div class="px-4 pb-4 space-y-3" style="border-top: 1px solid var(--border); padding-top: 12px;">
            <!-- Note about paid access -->
            <p class="text-[11px]" style="color: var(--fg3);">{svc.note}</p>

            <!-- Step 1 -->
            <div class="flex gap-2.5">
              <span class="shrink-0 w-4 h-4 rounded-full text-[9px] font-bold flex items-center justify-center mt-0.5"
                    style="background: var(--bg3); color: var(--fg3);">1</span>
              <div class="flex-1">
                <button
                  class="w-full text-left text-xs px-3 py-2.5 rounded-lg transition-colors font-medium"
                  style="background: var(--btn); color: var(--fg2);"
                  onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
                  onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'var(--btn)'}
                  onclick={() => open(svc.keyPage)}
                >
                  Open {svc.org} API Keys ↗
                </button>
              </div>
            </div>

            <!-- Step 2 -->
            <div class="flex gap-2.5">
              <span class="shrink-0 w-4 h-4 rounded-full text-[9px] font-bold flex items-center justify-center mt-0.5"
                    style="background: var(--bg3); color: var(--fg3);">2</span>
              <div class="flex-1 space-y-2">
                <p class="text-xs" style="color: var(--fg3);">Paste your API key:</p>
                <div class="flex gap-2">
                  <input
                    type="password"
                    class="flex-1 rounded-lg px-3 py-2 text-xs font-mono outline-none transition-colors"
                    style="background: var(--bg); border: 1px solid var(--border); color: var(--fg);"
                    onfocus={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--fg3)'}
                    onblur={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--border)'}
                    placeholder={svc.keyHint}
                    bind:value={keyInput[svc.id]}
                    onkeydown={(e) => e.key === 'Enter' && saveKey(svc.id)}
                    autofocus
                  />
                  <button
                    class="px-3 py-2 rounded-lg text-xs font-semibold transition-colors disabled:opacity-40"
                    style="background: rgba(34,197,94,0.15); color: #22c55e;"
                    onclick={() => saveKey(svc.id)}
                    disabled={saving[svc.id] || !keyInput[svc.id]?.trim()}
                  >
                    {saving[svc.id] ? '…' : 'Save'}
                  </button>
                </div>
              </div>
            </div>

            {#if isConnected}
              <button
                class="text-[11px] transition-colors"
                style="color: var(--fg3);"
                onmouseenter={(e) => (e.currentTarget as HTMLElement).style.color = '#ef4444'}
                onmouseleave={(e) => (e.currentTarget as HTMLElement).style.color = 'var(--fg3)'}
                onclick={() => disconnect(svc.id)}
              >
                Disconnect →
              </button>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="py-3 text-center text-[10px]" style="color: var(--fg3); border-top: 1px solid var(--border);">
    {connected.size} of {SERVICES.length} connected
  </div>
</div>
