pub mod claude;
pub mod claude_web;
pub mod chatgpt_web;
pub mod gemini;
pub mod grok;
pub mod openai;
pub mod perplexity;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Green,
    Yellow,
    Red,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub label: String,
    pub used: u64,
    pub limit: u64,
    pub reset_in_secs: Option<u64>,
}

/// Rich plan-level usage data fetched from the web app sessions
/// (claude.ai / chatgpt.com) rather than the raw API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanUsage {
    pub plan: Option<String>,

    // ── Claude.ai ───────────────────────────────────────────────────────
    /// Current session: percentage of quota *used* (0–100)
    pub session_pct_used: Option<f64>,
    /// Seconds until the session window resets
    pub session_resets_secs: Option<i64>,
    /// Weekly all-models: percentage *used* (0–100)
    pub weekly_pct_used: Option<f64>,
    /// Seconds until the weekly window resets
    pub weekly_resets_secs: Option<i64>,
    /// Claude Design: percentage *used* (0–100)
    pub design_pct_used: Option<f64>,
    pub routine_runs_used: Option<u32>,
    pub routine_runs_limit: Option<u32>,
    pub credits_spent: Option<f64>,
    pub credits_limit: Option<f64>,
    pub credits_balance: Option<f64>,
    pub credits_currency: Option<String>,

    // ── ChatGPT ─────────────────────────────────────────────────────────
    pub cap_5h_remaining: Option<u32>,
    pub cap_5h_limit: Option<u32>,
    pub cap_5h_resets_secs: Option<i64>,
    pub cap_weekly_remaining: Option<u32>,
    pub cap_weekly_limit: Option<u32>,
    pub cap_weekly_resets_secs: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceData {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub status: ServiceStatus,
    pub health_percent: Option<f64>,
    pub limits: Vec<RateLimit>,
    pub reset_date: Option<String>,
    pub last_updated: DateTime<Utc>,
    pub error: Option<String>,
    pub dashboard_url: String,
    /// Present only for web-session services (claude_web, chatgpt_web)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_usage: Option<PlanUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusThresholds {
    pub yellow_below: f64,
    pub red_below: f64,
}

impl Default for StatusThresholds {
    fn default() -> Self {
        Self {
            yellow_below: 50.0,
            red_below: 20.0,
        }
    }
}

impl ServiceData {
    pub fn compute_status(&mut self, thresholds: &StatusThresholds) {
        if self.error.is_some() {
            self.status = ServiceStatus::Unknown;
            return;
        }
        self.status = match self.health_percent {
            Some(p) if p > thresholds.yellow_below => ServiceStatus::Green,
            Some(p) if p > thresholds.red_below => ServiceStatus::Yellow,
            Some(_) => ServiceStatus::Red,
            None => ServiceStatus::Unknown,
        };
    }
}

#[async_trait]
pub trait AiService: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn icon(&self) -> &str;
    fn dashboard_url(&self) -> &str;

    async fn fetch(&self, api_key: &str, thresholds: &StatusThresholds) -> ServiceData;
}

pub fn build_error_data(
    id: &str,
    name: &str,
    icon: &str,
    dashboard_url: &str,
    err: String,
) -> ServiceData {
    ServiceData {
        id: id.to_string(),
        name: name.to_string(),
        icon: icon.to_string(),
        status: ServiceStatus::Unknown,
        health_percent: None,
        limits: vec![],
        reset_date: None,
        last_updated: Utc::now(),
        error: Some(err),
        dashboard_url: dashboard_url.to_string(),
        plan_usage: None,
    }
}
