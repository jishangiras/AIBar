<script lang="ts">
  import type { ServiceData } from '$lib/types';
  import StatusDot from './StatusDot.svelte';
  import ProgressBar from './ProgressBar.svelte';

  let { service, onclick }: { service: ServiceData; onclick: () => void } = $props();

  const icons: Record<string, string> = {
    claude: '✦',
    openai: '⊕',
    gemini: '◈',
    grok: '✕',
    perplexity: '⊗',
    claude_web: '✦',
    chatgpt_web: '⊕',
  };

  const updatedLabel = $derived.by(() => {
    const d = new Date(service.last_updated);
    const diffMin = Math.floor((Date.now() - d.getTime()) / 60000);
    if (diffMin < 1) return 'just now';
    if (diffMin < 60) return `${diffMin}m ago`;
    return `${Math.floor(diffMin / 60)}h ago`;
  });

  const percentLabel = $derived(
    service.health_percent != null
      ? `${Math.round(service.health_percent)}%`
      : service.error ? 'Error' : '—'
  );

  const iconChar = $derived(icons[service.icon] ?? icons[service.id] ?? '◉');
</script>

<button
  class="group relative flex flex-col gap-2 p-3 rounded-xl
         transition-all duration-200 ease-out
         hover:scale-[1.02] active:scale-[0.98]"
  style="background: var(--bg2); border: 1px solid var(--border);"
  onmouseenter={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--fg3)'}
  onmouseleave={(e) => (e.currentTarget as HTMLElement).style.borderColor = 'var(--border)'}
  onclick={onclick}
  title={service.error ?? service.name}
>
  <!-- Header row -->
  <div class="flex items-center justify-between">
    <span class="text-lg leading-none">{iconChar}</span>
    <StatusDot status={service.status} size="sm" pulse={service.status === 'red'} />
  </div>

  <!-- Service name -->
  <span class="text-xs font-medium truncate leading-none" style="color: var(--fg2);">
    {service.name}
  </span>

  <!-- Health percentage -->
  <span class="text-xl font-bold leading-none tabular-nums" style="color: var(--fg);">
    {percentLabel}
  </span>

  <!-- Progress bar -->
  {#if service.health_percent != null}
    <ProgressBar percent={service.health_percent} status={service.status} />
  {:else}
    <div class="w-full h-1 rounded-full" style="background: var(--border);"></div>
  {/if}

  <!-- Timestamp -->
  <span class="text-[10px] leading-none" style="color: var(--fg3);">{updatedLabel}</span>

  <!-- Error overlay -->
  {#if service.error}
    <div class="absolute inset-0 rounded-xl flex items-end p-2"
         style="background: rgba(239,68,68,0.06); border: 1px solid rgba(239,68,68,0.2);">
      <span class="text-[9px] text-red-500 leading-tight line-clamp-2">{service.error}</span>
    </div>
  {/if}
</button>
