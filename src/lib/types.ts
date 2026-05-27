export type ServiceStatus = 'green' | 'yellow' | 'red' | 'unknown';

export interface RateLimit {
  label: string;
  used: number;
  limit: number;
  reset_in_secs: number | null;
}

export interface ServiceData {
  id: string;
  name: string;
  icon: string;
  status: ServiceStatus;
  health_percent: number | null;
  limits: RateLimit[];
  reset_date: string | null;
  last_updated: string;
  error: string | null;
  dashboard_url: string;
}

export interface ServiceConfig {
  enabled: boolean;
  thresholds: {
    yellow_below: number;
    red_below: number;
  };
}

export interface AppSettings {
  poll_interval_secs: number;
  service_configs: Record<string, ServiceConfig>;
  launch_at_login: boolean;
}
