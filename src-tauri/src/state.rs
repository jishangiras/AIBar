use crate::services::{ServiceData, StatusThresholds};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceConfig {
    pub enabled: bool,
    pub thresholds: StatusThresholds,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            thresholds: StatusThresholds::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub poll_interval_secs: u64,
    pub service_configs: HashMap<String, ServiceConfig>,
    pub launch_at_login: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        let mut service_configs = HashMap::new();
        for id in ["claude", "claude_web", "openai", "chatgpt_web", "gemini", "grok", "perplexity"] {
            service_configs.insert(id.to_string(), ServiceConfig::default());
        }
        Self {
            poll_interval_secs: 600,
            service_configs,
            launch_at_login: false,
        }
    }
}

// Clone is cheap: all fields are Arc<RwLock<_>> — cloning just bumps refcounts.
#[derive(Clone)]
pub struct AppState {
    pub service_data: Arc<RwLock<HashMap<String, ServiceData>>>,
    pub settings: Arc<RwLock<AppSettings>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            service_data: Arc::new(RwLock::new(HashMap::new())),
            settings: Arc::new(RwLock::new(AppSettings::default())),
        }
    }
}
