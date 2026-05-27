<script lang="ts">
  import type { ServiceData, PlanUsage } from '$lib/types';
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

  function formatReset(secs: number | null | undefined): string {
    if (!secs) return '';
    const h = Math.floor(secs / 3600);
    const m = Math.ceil((secs % 3600) / 60);
    if (h > 0) return `${h}h ${m}m`;
    if (secs < 60) return `${secs}s`;
    return `${m}m`;
  }

  function formatNum(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(0)}k`;
    return String(n);
  }

  function fmtMoney(n: number | null | undefined, currency: string | null | undefined): string {
    if (n == null) return '—';
    const sym = currency === 'CAD' ? 'CA$' : currency === 'GBP' ? '£' : '$';
    return `${sym}${n.toFixed(2)}`;
  }

  const updatedFull = $derived(new Date(service.last_updated).toLocaleTimeString());
  const pu = $derived(service.plan_usage ?? null);
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
          <div class="flex items-center gap-2">
            <span class="text-[11px] font-semibold uppercase tracking-wider" style="color: var(--fg3);">Overall</span>
            {#if pu?.plan}
              <span class="text-[10px] px-1.5 py-0.5 rounded-full font-medium"
                    style="background: var(--bg3); color: var(--fg3);">{pu.plan}</span>
            {/if}
          </div>
          <span class="text-sm font-bold tabular-nums" style="color: var(--fg);">
            {Math.round(service.health_percent)}% remaining
          </span>
        </div>
        <ProgressBar percent={service.health_percent} status={service.status} />
      </div>
    {/if}

    <!-- ── Plan usage (claude_web / chatgpt_web) ─────────────────────── -->
    {#if pu}
      <!-- Claude.ai plan limits -->
      {#if pu.session_pct_used != null || pu.weekly_pct_used != null}
        <div class="space-y-3">
          <span class="text-[11px] font-semibold uppercase tracking-wider" style="color: var(--fg3);">Plan Limits</span>

          {#if pu.session_pct_used != null}
            {@const pct = 100 - pu.session_pct_used}
            {@const st = pct > 50 ? 'green' : pct > 20 ? 'yellow' : 'red'}
            <div class="space-y-1.5">
              <div class="flex items-center justify-between gap-2">
                <span class="text-xs" style="color: var(--fg2);">Current session</span>
                <span class="text-xs tabular-nums font-medium" style="color: var(--fg);">
                  {Math.round(pu.session_pct_used)}% used
                  {#if pu.session_resets_secs}
                    <span style="color: var(--fg3);"> · resets in {formatReset(pu.session_resets_secs)}</span>
                  {/if}
                </span>
              </div>
              <ProgressBar percent={pct} status={st} />
            </div>
          {/if}

          {#if pu.weekly_pct_used != null}
            {@const pct = 100 - pu.weekly_pct_used}
            {@const st = pct > 50 ? 'green' : pct > 20 ? 'yellow' : 'red'}
            <div class="space-y-1.5">
              <div class="flex items-center justify-between gap-2">
                <span class="text-xs" style="color: var(--fg2);">Weekly · all models</span>
                <span class="text-xs tabular-nums font-medium" style="color: var(--fg);">
                  {Math.round(pu.weekly_pct_used)}% used
                  {#if pu.weekly_resets_secs}
                    <span style="color: var(--fg3);"> · resets in {formatReset(pu.weekly_resets_secs)}</span>
                  {/if}
                </span>
              </div>
              <ProgressBar percent={pct} status={st} />
            </div>
          {/if}

          {#if pu.design_pct_used != null}
            {@const pct = 100 - pu.design_pct_used}
            {@const st = pct > 50 ? 'green' : pct > 20 ? 'yellow' : 'red'}
            <div class="space-y-1.5">
              <div class="flex items-center justify-between gap-2">
                <span class="text-xs" style="color: var(--fg2);">Claude Design</span>
                <span class="text-xs tabular-nums font-medium" style="color: var(--fg);">
                  {Math.round(pu.design_pct_used)}% used
                </span>
              </div>
              <ProgressBar percent={pct} status={st} />
            </div>
          {/if}
        </div>
      {/if}

      <!-- Claude.ai routine runs -->
      {#if pu.routine_runs_used != null && pu.routine_runs_limit != null}
        <div class="rounded-lg px-3 py-2.5 flex items-center justify-between"
             style="background: var(--bg2); border: 1px solid var(--border);">
          <span class="text-xs" style="color: var(--fg2);">Daily routine runs</span>
          <span class="text-sm font-bold tabular-nums" style="color: var(--fg);">
            {pu.routine_runs_used} / {pu.routine_runs_limit}
          </span>
        </div>
      {/if}

      <!-- ChatGPT message caps -->
      {#if pu.cap_5h_remaining != null || pu.cap_weekly_remaining != null}
        <div class="space-y-3">
          <span class="text-[11px] font-semibold uppercase tracking-wider" style="color: var(--fg3);">Message Caps</span>

          {#if pu.cap_5h_remaining != null && pu.cap_5h_limit != null}
            {@const pct = pu.cap_5h_limit > 0 ? pu.cap_5h_remaining / pu.cap_5h_limit * 100 : 100}
            {@const st = pct > 50 ? 'green' : pct > 20 ? 'yellow' : 'red'}
            <div class="space-y-1.5">
              <div class="flex items-center justify-between gap-2">
                <span class="text-xs" style="color: var(--fg2);">5-hour window</span>
                <span class="text-xs tabular-nums font-medium" style="color: var(--fg);">
                  {pu.cap_5h_remaining}/{pu.cap_5h_limit} remaining
                  {#if pu.cap_5h_resets_secs}
                    <span style="color: var(--fg3);"> · {formatReset(pu.cap_5h_resets_secs)}</span>
                  {/if}
                </span>
              </div>
              <ProgressBar percent={pct} status={st} />
            </div>
          {/if}

          {#if pu.cap_weekly_remaining != null && pu.cap_weekly_limit != null}
            {@const pct = pu.cap_weekly_limit > 0 ? pu.cap_weekly_remaining / pu.cap_weekly_limit * 100 : 100}
            {@const st = pct > 50 ? 'green' : pct > 20 ? 'yellow' : 'red'}
            <div class="space-y-1.5">
              <div class="flex items-center justify-between gap-2">
                <span class="text-xs" style="color: var(--fg2);">Weekly window</span>
                <span class="text-xs tabular-nums font-medium" style="color: var(--fg);">
                  {pu.cap_weekly_remaining}/{pu.cap_weekly_limit} remaining
                  {#if pu.cap_weekly_resets_secs}
                    <span style="color: var(--fg3);"> · resets in {formatReset(pu.cap_weekly_resets_secs)}</span>
                  {/if}
                </span>
              </div>
              <ProgressBar percent={pct} status={st} />
            </div>
          {/if}
        </div>
      {/if}

      <!-- Claude.ai credits card -->
      {#if pu.credits_spent != null || pu.credits_balance != null}
        <div class="rounded-lg p-3 space-y-2" style="background: var(--bg2); border: 1px solid var(--border);">
          <span class="text-[11px] font-semibold uppercase tracking-wider" style="color: var(--fg3);">Usage Credits</span>
          <div class="grid grid-cols-3 gap-2 mt-1">
            {#if pu.credits_spent != null}
              <div class="text-center">
                <p class="text-xs font-bold tabular-nums" style="color: var(--fg);">
                  {fmtMoney(pu.credits_spent, pu.credits_currency)}
                </p>
                <p class="text-[10px] mt-0.5" style="color: var(--fg3);">spent</p>
              </div>
            {/if}
            {#if pu.credits_limit != null}
              <div class="text-center">
                <p class="text-xs font-bold tabular-nums" style="color: var(--fg);">
                  {fmtMoney(pu.credits_limit, pu.credits_currency)}
                </p>
                <p class="text-[10px] mt-0.5" style="color: var(--fg3);">monthly limit</p>
              </div>
            {/if}
            {#if pu.credits_balance != null}
              <div class="text-center">
                <p class="text-xs font-bold tabular-nums" style="color: var(--status-green);">
                  {fmtMoney(pu.credits_balance, pu.credits_currency)}
                </p>
                <p class="text-[10px] mt-0.5" style="color: var(--fg3);">balance</p>
              </div>
            {/if}
          </div>
          {#if pu.credits_spent != null && pu.credits_limit != null && pu.credits_limit > 0}
            {@const pct = pu.credits_spent / pu.credits_limit * 100}
            {@const st = pct < 50 ? 'green' : pct < 80 ? 'yellow' : 'red'}
            <ProgressBar percent={pct} status={st} />
            <p class="text-[10px] text-right tabular-nums" style="color: var(--fg3);">
              {Math.round(pct)}% of monthly limit
            </p>
          {/if}
        </div>
      {/if}
    {/if}

    <!-- ── API rate-limit rows (non-web services) ─────────────────────── -->
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
    {:else if !service.error && !pu}
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
