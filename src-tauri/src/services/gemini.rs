use super::{build_error_data, AiService, ServiceData, StatusThresholds};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;

pub struct GeminiService {
    client: Client,
}

impl GeminiService {
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
impl AiService for GeminiService {
    fn id(&self) -> &str {
        "gemini"
    }
    fn name(&self) -> &str {
        "Gemini"
    }
    fn icon(&self) -> &str {
        "gemini"
    }
    fn dashboard_url(&self) -> &str {
        "https://aistudio.google.com/app/apikey"
    }

    async fn fetch(&self, api_key: &str, thresholds: &StatusThresholds) -> ServiceData {
        // Verify key by listing models — free operation
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models?key={api_key}&pageSize=1"
        );
        let res = self.client.get(&url).send().await;

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

        let status = res.status();
        if status == 400 || status == 401 || status == 403 {
            return build_error_data(
                self.id(),
                self.name(),
                self.icon(),
                self.dashboard_url(),
                "Invalid API key".to_string(),
            );
        }

        // Gemini free tier doesn't expose granular rate-limit headers;
        // show key-valid status at 100% until a richer usage endpoint is available
        let health_percent = if status.is_success() { Some(100.0) } else { None };

        let mut data = ServiceData {
            id: self.id().to_string(),
            name: self.name().to_string(),
            icon: self.icon().to_string(),
            status: super::ServiceStatus::Unknown,
            health_percent,
            limits: vec![],
            reset_date: None,
            last_updated: Utc::now(),
            error: if !status.is_success() {
                Some(format!("API error: {status}"))
            } else {
                None
            },
            dashboard_url: self.dashboard_url().to_string(),
            plan_usage: None,
        };
        data.compute_status(thresholds);
        data
    }
}
