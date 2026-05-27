import { invoke } from '@tauri-apps/api/core';
import type { AppSettings, ServiceData } from '$lib/types';

export const api = {
  getAllServices: () => invoke<ServiceData[]>('get_all_services'),
  refreshService: (serviceId: string) => invoke<ServiceData>('refresh_service', { serviceId }),
  getAggregateStatus: () => invoke<string>('get_aggregate_status'),

  getSettings: () => invoke<AppSettings>('get_settings'),
  saveSettings: (settings: AppSettings) => invoke<void>('save_settings', { settings }),

  saveApiKey: (serviceId: string, apiKey: string) =>
    invoke<void>('save_api_key', { serviceId, apiKey }),
  deleteApiKey: (serviceId: string) => invoke<void>('delete_api_key', { serviceId }),
  getStoredServiceIds: () => invoke<string[]>('get_stored_service_ids'),
};
