use super::{build_error_data, AiService, RateLimit, ServiceData, StatusThresholds};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;

pub struct PerplexityService {
    client: Client,
}

impl PerplexityService {
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
impl AiService for PerplexityService {
    fn id(&self) -> &str {
        "perplexity"
    }
    fn name(&self) -> &str {
        "Perplexity"
    }
    fn icon(&self) -> &str {
        "perplexity"
    }
    fn dashboard_url(&self) -> &str {
        "https://www.perplexity.ai/settings/api"
    }

    async fn fetch(&self, api_key: &str, thresholds: &StatusThresholds) -> ServiceData {
        let res = self
            .client
            .post("https://api.perplexity.ai/chat/completions")
            .bearer_auth(api_key)
            .header("content-type", "application/json")
            .body(r#"{"model":"sonar","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#)
            .send()
            .await;

        let res = match res {
            Ok(r) => r,
            Err(e) => {
                return build_error_data(
                    self.id(),
                    self.name(),
                    self.icon(),
                    self.dashboard_url(),
                    format!("Network error: {e}"),
                )
            }
        };

        if res.status() == 401 {
            return build_error_data(
                self.id(),
                self.name(),
                self.icon(),
                self.dashboard_url(),
                "Invalid API key".to_string(),
            );
        }

        let headers = res.headers();
        let mut limits = vec![];

        if let (Some(limit), Some(remaining)) = (
            parse_u64(headers, "x-ratelimit-limit-requests"),
            parse_u64(headers, "x-ratelimit-remaining-requests"),
        ) {
            limits.push(RateLimit {
                label: "Requests / min".to_string(),
                used: limit.saturating_sub(remaining),
                limit,
                reset_in_secs: None,
            });
        }

        let health_percent = if limits.is_empty() {
            if res.status().is_success() { Some(100.0) } else { None }
        } else {
            limits
                .iter()
                .map(|l| {
                    if l.limit == 0 {
                        100.0
                    } else {
                        let rem = l.limit.saturating_sub(l.used) as f64;
                        (rem / l.limit as f64) * 100.0
                    }
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

fn parse_u64(headers: &reqwest::header::HeaderMap, key: &str) -> Option<u64> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
}
