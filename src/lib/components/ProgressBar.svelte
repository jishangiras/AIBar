<script lang="ts">
  import type { ServiceStatus } from '$lib/types';

  let { percent, status }: { percent: number; status: ServiceStatus | string } = $props();

  const trackColors: Record<string, string> = {
    green:   '#22c55e',
    yellow:  '#f59e0b',
    red:     '#ef4444',
    unknown: '#888888',
  };

  const clamped = $derived(Math.max(0, Math.min(100, percent)));
  const fillColor = $derived(trackColors[status] ?? '#888888');
</script>

<div class="w-full h-1 rounded-full overflow-hidden" style="background: var(--border);">
  <div
    class="h-full rounded-full transition-all duration-500 ease-out"
    style="width: {clamped}%; background: {fillColor};"
  ></div>
</div>
