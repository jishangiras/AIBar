use super::{build_error_data, AiService, RateLimit, ServiceData, StatusThresholds};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;

pub struct OpenAiService {
    client: Client,
}

impl OpenAiService {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build reqwest client"),
        }
    }
}

#[async_trait]
impl AiService for OpenAiService {
    fn id(&self) -> &str { "openai" }
    fn name(&self) -> &str { "ChatGPT" }
    fn icon(&self) -> &str { "openai" }
    fn dashboard_url(&self) -> &str { "https://platform.openai.com/usage" }

    async fn fetch(&self, api_key: &str, thresholds: &StatusThresholds) -> ServiceData {
        let res = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
            .header("content-type", "application/json")
            .body(r#"{"model":"gpt-4o-mini","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#)
            .send()
            .await;

        let res = match res {
            Ok(r) => r,
            Err(e) => return self.err(&format!("Network error: {e}")),
        };

        let status = res.status();

        // Extract rate-limit headers before consuming body
        let req_limit     = parse_u64(res.headers(), "x-ratelimit-limit-requests");
        let req_remaining = parse_u64(res.headers(), "x-ratelimit-remaining-requests");
        let req_reset     = parse_duration(res.headers(), "x-ratelimit-reset-requests");
        let tok_limit     = parse_u64(res.headers(), "x-ratelimit-limit-tokens");
        let tok_remaining = parse_u64(res.headers(), "x-ratelimit-remaining-tokens");
        let tok_reset     = parse_duration(res.headers(), "x-ratelimit-reset-tokens");

        // Surface API errors with a clear human-readable message
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            let msg = openai_error_message(status.as_u16(), &body);
            return self.err(&msg);
        }

        let mut limits = vec![];
        if let (Some(limit), Some(remaining)) = (req_limit, req_remaining) {
            limits.push(RateLimit {
                label: "Requests / min".to_string(),
                used: limit.saturating_sub(remaining),
                limit,
                reset_in_secs: req_reset,
            });
        }
        if let (Some(limit), Some(remaining)) = (tok_limit, tok_remaining) {
            limits.push(RateLimit {
                label: "Tokens / min".to_string(),
                used: limit.saturating_sub(remaining),
                limit,
                reset_in_secs: tok_reset,
            });
        }

        let health_percent = if limits.is_empty() {
            Some(100.0) // 200 OK but no rate-limit headers → key valid, no quota info
        } else {
            limits
                .iter()
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
        };
        data.compute_status(thresholds);
        data
    }
}

impl OpenAiService {
    fn err(&self, msg: &str) -> ServiceData {
        build_error_data(self.id(), self.name(), self.icon(), self.dashboard_url(), msg.to_string())
    }
}

/// Turn an OpenAI HTTP error into a short human-readable string.
fn openai_error_message(status: u16, body: &str) -> String {
    // Check error code first — gives the clearest message
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(code) = v["error"]["code"].as_str() {
            return match code {
                "insufficient_quota" =>
                    "No API credits. The OpenAI API needs separate billing from ChatGPT Plus — add credits at platform.openai.com/billing".to_string(),
                "invalid_api_key" =>
                    "Invalid API key — check it at platform.openai.com/api-keys".to_string(),
                "model_not_found" =>
                    "Model unavailable on this account tier".to_string(),
                _ => format!("API error: {code}"),
            };
        }
    }
    match status {
        401 => "Invalid API key — check it at platform.openai.com/api-keys".to_string(),
        402 | 429 => "No API credits. Add billing at platform.openai.com/billing (separate from ChatGPT Plus)".to_string(),
        403 => "Access denied — check API key permissions".to_string(),
        _   => format!("HTTP {status} error from OpenAI"),
    }
}

fn parse_u64(headers: &reqwest::header::HeaderMap, key: &str) -> Option<u64> {
    headers.get(key)?.to_str().ok()?.parse().ok()
}

fn parse_duration(headers: &reqwest::header::HeaderMap, key: &str) -> Option<u64> {
    let s = headers.get(key)?.to_str().ok()?;
    let mut secs = 0u64;
    let mut buf = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            buf.push(ch);
        } else {
            let n: u64 = buf.parse().unwrap_or(0);
            buf.clear();
            secs += match ch { 'h' => n * 3600, 'm' => n * 60, 's' => n, _ => 0 };
        }
    }
    Some(secs)
}
