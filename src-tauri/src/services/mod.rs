pub mod claude;
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
    }
}
