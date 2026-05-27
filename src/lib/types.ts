export type ServiceStatus = 'green' | 'yellow' | 'red' | 'unknown';

export interface RateLimit {
  label: string;
  used: number;
  limit: number;
  reset_in_secs: number | null;
}

/** Rich plan-level usage — present only for claude_web / chatgpt_web services */
export interface PlanUsage {
  plan: string | null;

  // ── Claude.ai ──────────────────────────────────────────────────────────
  /** Percentage of session quota *used* (0–100) */
  session_pct_used: number | null;
  /** Seconds until the session window resets */
  session_resets_secs: number | null;
  /** Percentage of weekly quota *used* (0–100) */
  weekly_pct_used: number | null;
  /** Seconds until the weekly window resets */
  weekly_resets_secs: number | null;
  /** Claude Design percentage *used* (0–100) */
  design_pct_used: number | null;
  routine_runs_used: number | null;
  routine_runs_limit: number | null;
  credits_spent: number | null;
  credits_limit: number | null;
  credits_balance: number | null;
  credits_currency: string | null;

  // ── ChatGPT ────────────────────────────────────────────────────────────
  cap_5h_remaining: number | null;
  cap_5h_limit: number | null;
  cap_5h_resets_secs: number | null;
  cap_weekly_remaining: number | null;
  cap_weekly_limit: number | null;
  cap_weekly_resets_secs: number | null;
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
  /** Only set for web-session services */
  plan_usage?: PlanUsage | null;
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
