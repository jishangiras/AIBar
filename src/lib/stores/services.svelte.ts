import { listen } from '@tauri-apps/api/event';
import { api } from '$lib/api/tauri';
import type { ServiceData } from '$lib/types';

function createServicesStore() {
  let services = $state<ServiceData[]>([]);
  let loading = $state(true);
  let selectedId = $state<string | null>(null);

  async function load() {
    loading = true;
    try {
      services = await api.getAllServices();
    } finally {
      loading = false;
    }
  }

  async function refresh(id?: string) {
    if (id) {
      const updated = await api.refreshService(id);
      services = services.map((s) => (s.id === id ? updated : s));
    } else {
      await load();
    }
  }

  async function startListening() {
    await listen<ServiceData>('service-updated', (event) => {
      const updated = event.payload;
      const idx = services.findIndex((s) => s.id === updated.id);
      if (idx >= 0) {
        services = [...services.slice(0, idx), updated, ...services.slice(idx + 1)];
      } else {
        services = [...services, updated];
      }
    });
  }

  function select(id: string | null) {
    selectedId = id;
  }

  const selected = $derived(services.find((s) => s.id === selectedId) ?? null);

  return {
    get services() { return services; },
    get loading() { return loading; },
    get selectedId() { return selectedId; },
    get selected() { return selected; },
    load,
    refresh,
    startListening,
    select,
  };
}

export const servicesStore = createServicesStore();
