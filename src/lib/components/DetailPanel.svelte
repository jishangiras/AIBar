<script lang="ts">
  import type { ServiceData } from '$lib/types';
  import StatusDot from './StatusDot.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import { open } from '@tauri-apps/plugin-shell';

  let {
    service,
    onclose,
    onrefresh,
    refreshing = false,
  }: {
    service: ServiceData;
    onclose: () => void;
    onrefresh: () => void;
    refreshing?: boolean;
  } = $props();

  const icons: Record<string, string> = {
    claude: '✦', openai: '⊕', gemini: '◈', grok: '✕', perplexity: '⊗',
  };

  const iconChar = $derived(icons[service.icon] ?? icons[service.id] ?? '◉');

  function formatReset(secs: number | null): string {
    if (!secs) return '';
    if (secs < 60) return `${secs}s`;
    return `${Math.ceil(secs / 60)}m`;
  }

  function formatNum(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(0)}k`;
    return String(n);
  }

  const updatedFull = $derived(new Date(service.last_updated).toLocaleTimeString());
</script>

<div
  class="absolute inset-0 z-10 animate-slide-in flex flex-col rounded-xl"
  style="background: var(--bg); border: 1px solid var(--border);"
>
  <!-- Header -->
  <div class="flex items-center gap-3 p-4" style="border-bottom: 1px solid var(--border);">
    <button
      class="transition-colors p-1.5 rounded-lg text-sm"
      style="color: var(--fg3);"
      onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
      onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'transparent'}
      onclick={onclose}
      aria-label="Back"
    >←</button>
    <span class="text-xl">{iconChar}</span>
    <div class="flex-1 min-w-0">
      <h2 class="font-semibold text-sm" style="color: var(--fg);">{service.name}</h2>
      <p class="text-[11px]" style="color: var(--fg3);">Updated {updatedFull}</p>
    </div>
    <StatusDot status={service.status} size="lg" pulse={service.status === 'red'} />
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-y-auto p-4 space-y-4">
    {#if service.error}
      <div class="rounded-lg p-3" style="background: rgba(239,68,68,0.08); border: 1px solid rgba(239,68,68,0.2);">
        <p class="text-xs text-red-500">{service.error}</p>
      </div>
    {/if}

    {#if service.health_percent != null}
      <div class="space-y-2">
        <div class="flex items-center justify-between">
          <span class="text-[11px] font-semibold uppercase tracking-wider" style="color: var(--fg3);">Overall</span>
          <span class="text-sm font-bold tabular-nums" style="color: var(--fg);">
            {Math.round(service.health_percent)}%
          </span>
        </div>
        <ProgressBar percent={service.health_percent} status={service.status} />
      </div>
    {/if}

    {#if service.limits.length > 0}
      <div class="space-y-3">
        <span class="text-[11px] font-semibold uppercase tracking-wider" style="color: var(--fg3);">Rate Limits</span>
        {#each service.limits as limit}
          {@const remaining = limit.limit - limit.used}
          {@const pct = limit.limit > 0 ? (remaining / limit.limit) * 100 : 100}
          {@const limStatus = pct > 50 ? 'green' : pct > 20 ? 'yellow' : 'red'}
          <div class="space-y-1.5">
            <div class="flex items-center justify-between gap-2">
              <span class="text-xs truncate" style="color: var(--fg2);">{limit.label}</span>
              <span class="text-xs tabular-nums font-medium shrink-0" style="color: var(--fg);">
                {formatNum(remaining)}/{formatNum(limit.limit)}
                {#if limit.reset_in_secs}
                  <span style="color: var(--fg3);"> ↺{formatReset(limit.reset_in_secs)}</span>
                {/if}
              </span>
            </div>
            <ProgressBar percent={pct} status={limStatus} />
          </div>
        {/each}
      </div>
    {:else if !service.error}
      <p class="text-xs text-center py-4" style="color: var(--fg3);">
        No rate-limit data yet.<br/>Data appears after your first API call.
      </p>
    {/if}

    {#if service.reset_date}
      <p class="text-xs" style="color: var(--fg3);">Monthly reset: {service.reset_date}</p>
    {/if}
  </div>

  <!-- Footer -->
  <div class="p-3 flex gap-2" style="border-top: 1px solid var(--border);">
    <button
      class="flex-1 text-xs py-2 px-3 rounded-lg transition-colors disabled:opacity-40"
      style="background: var(--btn); color: var(--fg2);"
      onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
      onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'var(--btn)'}
      onclick={onrefresh}
      disabled={refreshing}
    >
      {refreshing ? 'Refreshing…' : 'Refresh'}
    </button>
    <button
      class="flex-1 text-xs py-2 px-3 rounded-lg transition-colors"
      style="background: var(--btn); color: var(--fg2);"
      onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = 'var(--btn-hover)'}
      onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'var(--btn)'}
      onclick={() => open(service.dashboard_url)}
    >
      Dashboard ↗
    </button>
  </div>
</div>
