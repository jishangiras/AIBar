use super::{build_error_data, AiService, PlanUsage, ServiceData, StatusThresholds};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::Value;

/// Monitors your **ChatGPT plan** usage (5-hour rolling window and
/// weekly message cap) via the chatgpt.com internal API.
///
/// Credential: the `__Secure-next-auth.session-token` cookie value
/// from chatgpt.com.  Retrieve it from your browser's
/// DevTools → Application → Cookies → chatgpt.com.
pub struct ChatGptWebService {
    client: Client,
}

impl ChatGptWebService {
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
impl AiService for ChatGptWebService {
    fn id(&self) -> &str { "chatgpt_web" }
    fn name(&self) -> &str { "ChatGPT" }
    fn icon(&self) -> &str { "openai" }
    fn dashboard_url(&self) -> &str { "https://chatgpt.com/" }

    async fn fetch(&self, session_token: &str, thresholds: &StatusThresholds) -> ServiceData {
        // ── Step 1: exchange session cookie for an access token ──────────
        let sess = self
            .client
            .get("https://chatgpt.com/api/auth/session")
            .header(
                "Cookie",
                format!("__Secure-next-auth.session-token={session_token}"),
            )
            .header("Referer", "https://chatgpt.com/")
            .header("Accept", "application/json")
            .send()
            .await;

        let sess_body: Value = match sess {
            Ok(r) if r.status().is_success() => match r.json().await {
                Ok(v) => v,
                Err(e) => return self.err(&format!("Failed to parse session: {e}")),
            },
            Ok(r) => {
                let s = r.status().as_u16();
                return self.err(match s {
                    401 | 403 => {
                        "Session expired — update your session token in Settings \
                         (DevTools → Application → Cookies → chatgpt.com)"
                    }
                    _ => "Session auth failed — check your session token",
                });
            }
            Err(e) => return self.err(&format!("Network error: {e}")),
        };

        let access_token = match sess_body["accessToken"].as_str() {
            Some(t) => t.to_string(),
            None => {
                return self.err(
                    "Invalid session token — log in to chatgpt.com, \
                     copy __Secure-next-auth.session-token from cookies",
                )
            }
        };

        // ── Step 2: fetch message-cap data ───────────────────────────────
        let cap_res = self
            .client
            .get("https://chatgpt.com/backend-api/accounts/check/v4-2023-04-27")
            .bearer_auth(&access_token)
            .header("Referer", "https://chatgpt.com/")
            .header("Accept", "application/json")
            .send()
            .await;

        let body: Value = match cap_res {
            Ok(r) if r.status().is_success() => match r.json().await {
                Ok(v) => v,
                Err(e) => return self.err(&format!("Failed to parse usage: {e}")),
            },
            Ok(r) => {
                return self.err(&format!("Usage check failed: HTTP {}", r.status()))
            }
            Err(e) => return self.err(&format!("Network error: {e}")),
        };

        // ── Plan type ────────────────────────────────────────────────────
        let plan = body["account_plan"]["plan_type"].as_str().map(|s| {
            match s {
                "chatgptfreeplan" => "Free".to_string(),
                "chatgptplusplan" => "Plus".to_string(),
                "chatgptteamplan" => "Team".to_string(),
                "chatgptenterpriseplan" => "Enterprise".to_string(),
                _ => s.to_string(),
            }
        });

        // ── Message caps (array of {cap_type, remaining, total, reset_at}) ─
        let mut cap_5h_remaining: Option<u32> = None;
        let mut cap_5h_limit: Option<u32> = None;
        let mut cap_5h_resets_secs: Option<i64> = None;
        let mut cap_weekly_remaining: Option<u32> = None;
        let mut cap_weekly_limit: Option<u32> = None;
        let mut cap_weekly_resets_secs: Option<i64> = None;

        // Try array form first
        if let Some(caps) = body["message_caps"].as_array() {
            for cap in caps {
                let cap_type = cap["cap_type"].as_str().unwrap_or("");
                let remaining = cap["remaining"].as_u64().map(|n| n as u32);
                let total     = cap["total"].as_u64().map(|n| n as u32);
                let resets    = resets_secs(cap["reset_at"].as_str());

                if cap_type.contains('5') || cap_type.contains("hour") {
                    cap_5h_remaining  = remaining;
                    cap_5h_limit      = total;
                    cap_5h_resets_secs = resets;
                } else if cap_type.contains("week") {
                    cap_weekly_remaining   = remaining;
                    cap_weekly_limit       = total;
                    cap_weekly_resets_secs = resets;
                }
            }
        }

        // Fall back to flat message_cap object
        if cap_5h_remaining.is_none() {
            cap_5h_remaining = body["message_cap"]["remaining_5h"]
                .as_u64().map(|n| n as u32);
            cap_5h_limit = body["message_cap"]["message_cap_5h"]
                .as_u64().map(|n| n as u32);
            cap_5h_resets_secs = resets_secs(
                body["message_cap"]["message_cap_5h_reset_at"].as_str(),
            );
        }
        if cap_weekly_remaining.is_none() {
            cap_weekly_remaining = body["message_cap"]["remaining_weekly"]
                .as_u64().map(|n| n as u32);
            cap_weekly_limit = body["message_cap"]["message_cap_weekly"]
                .as_u64().map(|n| n as u32);
            cap_weekly_resets_secs = resets_secs(
                body["message_cap"]["message_cap_weekly_reset_at"].as_str(),
            );
        }

        // ── Health: worst remaining % across 5h and weekly ───────────────
        let health_percent = {
            let h5 = pct_remaining(cap_5h_remaining, cap_5h_limit);
            let hw = pct_remaining(cap_weekly_remaining, cap_weekly_limit);
            match (h5, hw) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None)    => Some(a),
                (None, Some(b))    => Some(b),
                (None, None)       => Some(100.0),
            }
        };

        let plan_usage = PlanUsage {
            plan,
            session_pct_used: None,
            session_resets_secs: None,
            weekly_pct_used: None,
            weekly_resets_secs: None,
            design_pct_used: None,
            routine_runs_used: None,
            routine_runs_limit: None,
            credits_spent: None,
            credits_limit: None,
            credits_balance: None,
            credits_currency: None,
            cap_5h_remaining,
            cap_5h_limit,
            cap_5h_resets_secs,
            cap_weekly_remaining,
            cap_weekly_limit,
            cap_weekly_resets_secs,
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

fn pct_remaining(remaining: Option<u32>, limit: Option<u32>) -> Option<f64> {
    match (remaining, limit) {
        (Some(r), Some(l)) if l > 0 => Some(r as f64 / l as f64 * 100.0),
        _ => None,
    }
}
