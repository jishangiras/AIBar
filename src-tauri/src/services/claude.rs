use super::{build_error_data, AiService, RateLimit, ServiceData, StatusThresholds};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;

pub struct ClaudeService {
    client: Client,
}

impl ClaudeService {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build reqwest client"),
        }
    }
    fn err(&self, msg: &str) -> ServiceData {
        build_error_data(self.id(), self.name(), self.icon(), self.dashboard_url(), msg.to_string())
    }
}

#[async_trait]
impl AiService for ClaudeService {
    fn id(&self) -> &str { "claude" }
    fn name(&self) -> &str { "Claude" }
    fn icon(&self) -> &str { "claude" }
    fn dashboard_url(&self) -> &str { "https://console.anthropic.com/settings/usage" }

    async fn fetch(&self, api_key: &str, thresholds: &StatusThresholds) -> ServiceData {
        let res = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(r#"{"model":"claude-3-5-haiku-20241022","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#)
            .send()
            .await;

        let res = match res {
            Ok(r) => r,
            Err(e) => return self.err(&format!("Network error: {e}")),
        };

        let status = res.status();

        // Read all needed header values before potentially consuming the body
        let req_limit     = parse_header(res.headers(), "anthropic-ratelimit-requests-limit");
        let req_rem       = parse_header(res.headers(), "anthropic-ratelimit-requests-remaining");
        let req_reset     = parse_reset(res.headers(),  "anthropic-ratelimit-requests-reset");
        let tok_limit     = parse_header(res.headers(), "anthropic-ratelimit-tokens-limit");
        let tok_rem       = parse_header(res.headers(), "anthropic-ratelimit-tokens-remaining");
        let tok_reset     = parse_reset(res.headers(),  "anthropic-ratelimit-tokens-reset");
        let in_limit      = parse_header(res.headers(), "anthropic-ratelimit-input-tokens-limit");
        let in_rem        = parse_header(res.headers(), "anthropic-ratelimit-input-tokens-remaining");
        let out_limit     = parse_header(res.headers(), "anthropic-ratelimit-output-tokens-limit");
        let out_rem       = parse_header(res.headers(), "anthropic-ratelimit-output-tokens-remaining");

        // Surface errors with a clear message
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            let msg = claude_error_message(status.as_u16(), &body);
            return self.err(&msg);
        }

        let mut limits = vec![];

        if let (Some(limit), Some(remaining)) = (req_limit, req_rem) {
            limits.push(RateLimit { label: "Requests / min".into(),
                used: limit.saturating_sub(remaining), limit, reset_in_secs: req_reset });
        }
        if let (Some(limit), Some(remaining)) = (tok_limit, tok_rem) {
            limits.push(RateLimit { label: "Tokens / min".into(),
                used: limit.saturating_sub(remaining), limit, reset_in_secs: tok_reset });
        }
        if let (Some(limit), Some(remaining)) = (in_limit, in_rem) {
            limits.push(RateLimit { label: "Input tokens / min".into(),
                used: limit.saturating_sub(remaining), limit, reset_in_secs: None });
        }
        if let (Some(limit), Some(remaining)) = (out_limit, out_rem) {
            limits.push(RateLimit { label: "Output tokens / min".into(),
                used: limit.saturating_sub(remaining), limit, reset_in_secs: None });
        }

        let health_percent = if limits.is_empty() {
            Some(100.0) // 200 OK, key valid, no quota headers exposed
        } else {
            limits.iter()
                .map(|l| if l.limit == 0 { 100.0 } else {
                    l.limit.saturating_sub(l.used) as f64 / l.limit as f64 * 100.0
                })
                .reduce(f64::min)
        };

        let mut data = ServiceData {
            id: self.id().to_string(),
            name: self.name().to_string(),
            icon: self.icon().to_string(),
            status: super::ServiceStatus::Unknown,
            health_percent,
            limits,
            reset_date: None,
            last_updated: Utc::now(),
            error: None,
            dashboard_url: self.dashboard_url().to_string(),
            plan_usage: None,
        };
        data.compute_status(thresholds);
        data
    }
}

fn claude_error_message(status: u16, body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(err_type) = v["error"]["type"].as_str() {
            return match err_type {
                "authentication_error" =>
                    "Invalid API key — check it at console.anthropic.com/settings/keys".to_string(),
                "permission_error" =>
                    "API key lacks permissions — check console.anthropic.com".to_string(),
                "rate_limit_error" =>
                    "Rate limited — you're hitting the API rate limits".to_string(),
                "overloaded_error" =>
                    "Anthropic API overloaded — try again later".to_string(),
                _ => {
                    if let Some(msg) = v["error"]["message"].as_str() {
                        let s = if msg.len() > 100 { &msg[..100] } else { msg };
                        s.to_string()
                    } else {
                        format!("API error: {err_type}")
                    }
                }
            };
        }
    }
    match status {
        401 => "Invalid API key — check console.anthropic.com/settings/keys".to_string(),
        403 => "The Claude API requires a paid account at console.anthropic.com (separate from Claude.ai Pro)".to_string(),
        429 => "Rate limited or quota exceeded".to_string(),
        _   => format!("HTTP {status} — Claude API requires a paid account at console.anthropic.com"),
    }
}

fn parse_header(headers: &reqwest::header::HeaderMap, key: &str) -> Option<u64> {
    headers.get(key)?.to_str().ok()?.parse().ok()
}

fn parse_reset(headers: &reqwest::header::HeaderMap, key: &str) -> Option<u64> {
    headers.get(key)?.to_str().ok()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc).signed_duration_since(Utc::now()).num_seconds().max(0) as u64)
}
