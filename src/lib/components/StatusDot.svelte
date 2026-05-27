<script lang="ts">
  import type { ServiceStatus } from '$lib/types';

  let { status, size = 'md', pulse = false }: {
    status: ServiceStatus;
    size?: 'sm' | 'md' | 'lg';
    pulse?: boolean;
  } = $props();

  const colors: Record<ServiceStatus, string> = {
    green: 'bg-[#22c55e] shadow-[0_0_6px_#22c55e66]',
    yellow: 'bg-[#eab308] shadow-[0_0_6px_#eab30866]',
    red: 'bg-[#ef4444] shadow-[0_0_6px_#ef444466]',
    unknown: 'bg-[#555] shadow-none',
  };

  const sizes: Record<string, string> = {
    sm: 'w-1.5 h-1.5',
    md: 'w-2.5 h-2.5',
    lg: 'w-3.5 h-3.5',
  };
</script>

<span
  class="rounded-full inline-block shrink-0 transition-all duration-300
    {colors[status]}
    {sizes[size]}
    {pulse && status !== 'unknown' ? 'animate-pulse-slow' : ''}"
></span>
