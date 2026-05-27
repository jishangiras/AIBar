use super::{build_error_data, AiService, PlanUsage, ServiceData, StatusThresholds};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::Value;

/// Monitors your **claude.ai Pro plan** usage (session %, weekly %,
/// Claude Design %, daily routine runs, and usage credits) via the
/// claude.ai internal API.
///
/// Credential: the `sessionKey` cookie value from claude.ai
/// (looks like `sk-ant-sid…`). Retrieve it from your browser's
/// DevTools → Application → Cookies → claude.ai.
pub struct ClaudeWebService {
    client: Client,
}

impl ClaudeWebService {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent(
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
                     AppleWebKit/537.36 (KHTML, like Gecko) \
                     Chrome/124.0.0.0 Safari/537.36",
                )
                .build()
                .expect("failed to build reqwest client"),
        }
    }

    fn err(&self, msg: &str) -> ServiceData {
        build_error_data(
            self.id(),
            self.name(),
            self.icon(),
            self.dashboard_url(),
            msg.to_string(),
        )
    }
}

#[async_trait]
impl AiService for ClaudeWebService {
    fn id(&self) -> &str { "claude_web" }
    fn name(&self) -> &str { "Claude.ai" }
    fn icon(&self) -> &str { "claude" }
    fn dashboard_url(&self) -> &str { "https://claude.ai/settings/limits" }

    async fn fetch(&self, session_key: &str, thresholds: &StatusThresholds) -> ServiceData {
        let res = self
            .client
            .get("https://claude.ai/api/usage_limits")
            .header("Cookie", format!("sessionKey={session_key}"))
            .header("Referer", "https://claude.ai/settings/limits")
            .header("Accept", "application/json, text/plain, */*")
            .send()
            .await;

        let res = match res {
            Ok(r) => r,
            Err(e) => return self.err(&format!("Network error: {e}")),
        };

        match res.status().as_u16() {
            401 | 403 => {
                return self.err(
                    "Session expired — update your sessionKey in Settings \
                     (DevTools → Application → Cookies → claude.ai)",
                )
            }
            s if s >= 400 => {
                return self.err(&format!("HTTP {s} — check your session key"))
            }
            _ => {}
        }

        let body: Value = match res.json().await {
            Ok(v) => v,
            Err(e) => return self.err(&format!("Failed to parse response: {e}")),
        };

        // ── Plan ────────────────────────────────────────────────────────
        let plan = body["plan"].as_str()
            .or_else(|| body["account_plan"]["plan"].as_str())
            .map(String::from);

        // ── Session usage ────────────────────────────────────────────────
        let session_pct_used = body["session_limit"]["percent_used"].as_f64()
            .or_else(|| body["session"]["percent_used"].as_f64())
            .or_else(|| body["session_usage_pct"].as_f64());

        let session_resets_secs = resets_secs(
            body["session_limit"]["resets_at"].as_str()
                .or_else(|| body["session"]["resets_at"].as_str())
                .or_else(|| body["session_resets_at"].as_str()),
        );

        // ── Weekly usage ─────────────────────────────────────────────────
        let weekly_pct_used = body["weekly_limit"]["percent_used"].as_f64()
            .or_else(|| body["weekly"]["percent_used"].as_f64())
            .or_else(|| body["weekly_usage_pct"].as_f64());

        let weekly_resets_secs = resets_secs(
            body["weekly_limit"]["resets_at"].as_str()
                .or_else(|| body["weekly"]["resets_at"].as_str())
                .or_else(|| body["weekly_resets_at"].as_str()),
        );

        // ── Claude Design ────────────────────────────────────────────────
        let design_pct_used = body["claude_design_limit"]["percent_used"].as_f64()
            .or_else(|| body["claude_design"]["percent_used"].as_f64())
            .or_else(|| body["claude_design_pct"].as_f64());

        // ── Routine runs ─────────────────────────────────────────────────
        let routine_runs_used = body["routine_runs"]["used"].as_u64().map(|n| n as u32)
            .or_else(|| body["routine_runs_used"].as_u64().map(|n| n as u32));
        let routine_runs_limit = body["routine_runs"]["limit"].as_u64().map(|n| n as u32)
            .or_else(|| body["routine_runs_limit"].as_u64().map(|n| n as u32));

        // ── Credits ──────────────────────────────────────────────────────
        let credits_spent = body["usage_credits"]["spent_amount"].as_f64()
            .or_else(|| body["credits"]["spent"].as_f64())
            .or_else(|| body["credits_spent"].as_f64());
        let credits_limit = body["usage_credits"]["spend_limit"].as_f64()
            .or_else(|| body["credits"]["limit"].as_f64())
            .or_else(|| body["credits_limit"].as_f64());
        let credits_balance = body["usage_credits"]["current_balance"].as_f64()
            .or_else(|| body["credits"]["balance"].as_f64())
            .or_else(|| body["credits_balance"].as_f64());
        let credits_currency = body["usage_credits"]["currency"].as_str()
            .or_else(|| body["credits"]["currency"].as_str())
            .map(String::from);

        // ── Health: worst remaining capacity across session + weekly ─────
        let health_percent = {
            let s = session_pct_used.map(|p| 100.0 - p);
            let w = weekly_pct_used.map(|p| 100.0 - p);
            match (s, w) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None)    => Some(a),
                (None, Some(b))    => Some(b),
                (None, None)       => Some(100.0), // no data → assume healthy
            }
        };

        let plan_usage = PlanUsage {
            plan,
            session_pct_used,
            session_resets_secs,
            weekly_pct_used,
            weekly_resets_secs,
            design_pct_used,
            routine_runs_used,
            routine_runs_limit,
            credits_spent,
            credits_limit,
            credits_balance,
            credits_currency,
            cap_5h_remaining: None,
            cap_5h_limit: None,
            cap_5h_resets_secs: None,
            cap_weekly_remaining: None,
            cap_weekly_limit: None,
            cap_weekly_resets_secs: None,
        };

        let mut data = ServiceData {
            id: self.id().to_string(),
            name: self.name().to_string(),
            icon: self.icon().to_string(),
            status: super::ServiceStatus::Unknown,
            health_percent,
            limits: vec![],
            reset_date: None,
            last_updated: Utc::now(),
            error: None,
            dashboard_url: self.dashboard_url().to_string(),
            plan_usage: Some(plan_usage),
        };
        data.compute_status(thresholds);
        data
    }
}

fn resets_secs(s: Option<&str>) -> Option<i64> {
    let s = s?;
    let dt = chrono::DateTime::parse_from_rfc3339(s).ok()?;
    Some(
        dt.with_timezone(&Utc)
            .signed_duration_since(Utc::now())
            .num_seconds()
            .max(0),
    )
}
