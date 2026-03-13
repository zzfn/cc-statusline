use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::colors;

pub trait Provider {
    fn name(&self) -> &'static str;
    fn matches(&self, base_url: &str) -> bool;
    fn get_parts(&self, base_url: &str, auth_token: &str) -> Vec<String>;
}

/// 质普配额限制信息
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct QuotaLimit {
    #[serde(rename = "type")]
    pub limit_type: String,
    pub percentage: f64,
    #[serde(rename = "currentValue")]
    pub current_value: Option<u64>,
    pub usage: Option<u64>,
}

/// 质普使用情况缓存
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ZhipuUsageCache {
    pub token_limit: Option<QuotaLimit>,
    pub mcp_limit: Option<QuotaLimit>,
    pub timestamp: DateTime<Utc>,
}

pub struct ZhipuProvider;

impl ZhipuProvider {
    fn cache_path(&self) -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".claude").join(".zhipu_cache.json")
    }

    fn read_cache(&self) -> Option<ZhipuUsageCache> {
        let cache_path = self.cache_path();
        let content = fs::read_to_string(cache_path).ok()?;
        let cache: ZhipuUsageCache = serde_json::from_str(&content).ok()?;

        // 检查缓存是否过期（3分钟）
        let now = Utc::now();
        let age = now.signed_duration_since(cache.timestamp);
        if age.num_minutes() < 3 {
            Some(cache)
        } else {
            None
        }
    }

    fn write_cache(&self, cache: &ZhipuUsageCache) {
        let cache_path = self.cache_path();
        if let Ok(json) = serde_json::to_string(cache) {
            let _ = fs::write(cache_path, json);
        }
    }

    fn fetch_usage(&self, base_url: &str, auth_token: &str) -> Option<ZhipuUsageCache> {
        let parsed_url = base_url.parse::<reqwest::Url>().ok()?;
        let base_domain = format!("{}://{}", parsed_url.scheme(), parsed_url.host_str()?);
        let quota_url = format!("{}/api/monitor/usage/quota/limit", base_domain);

        let client = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .ok()?;

        let response = client
            .get(&quota_url)
            .header("Authorization", auth_token)
            .header("Accept-Language", "en-US,en")
            .header("Content-Type", "application/json")
            .send()
            .ok()?;

        if !response.status().is_success() {
            return None;
        }

        #[derive(Deserialize)]
        struct ApiResponse {
            data: ApiData,
        }

        #[derive(Deserialize)]
        struct ApiData {
            limits: Vec<QuotaLimit>,
        }

        let api_response: ApiResponse = response.json().ok()?;

        let mut token_limit = None;
        let mut mcp_limit = None;

        for limit in api_response.data.limits {
            match limit.limit_type.as_str() {
                "TOKENS_LIMIT" => token_limit = Some(limit),
                "TIME_LIMIT" => mcp_limit = Some(limit),
                _ => {}
            }
        }

        let cache = ZhipuUsageCache {
            token_limit,
            mcp_limit,
            timestamp: Utc::now(),
        };

        self.write_cache(&cache);
        Some(cache)
    }

    fn get_usage(&self, base_url: &str, auth_token: &str) -> Option<ZhipuUsageCache> {
        if !self.matches(base_url) {
            return None;
        }

        if let Some(cache) = self.read_cache() {
            return Some(cache);
        }

        self.fetch_usage(base_url, auth_token)
    }
}

impl Provider for ZhipuProvider {
    fn name(&self) -> &'static str {
        "zhipu"
    }

    fn matches(&self, base_url: &str) -> bool {
        base_url.contains("bigmodel.cn") || base_url.contains("z.ai")
    }

    fn get_parts(&self, base_url: &str, auth_token: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let Some(zhipu_usage) = self.get_usage(base_url, auth_token) else {
            return parts;
        };

        if let Some(ref token_limit) = zhipu_usage.token_limit {
            let color = if token_limit.percentage >= 80.0 {
                colors::RED
            } else if token_limit.percentage >= 60.0 {
                colors::YELLOW
            } else {
                colors::GREEN
            };
            parts.push(format!(
                "{}[ZAI] Token(5h):{:.0}%{}",
                color,
                token_limit.percentage,
                colors::RESET
            ));
        }

        if let Some(ref mcp_limit) = zhipu_usage.mcp_limit {
            let color = if mcp_limit.percentage >= 80.0 {
                colors::RED
            } else if mcp_limit.percentage >= 60.0 {
                colors::YELLOW
            } else {
                colors::GREEN
            };
            parts.push(format!(
                "{}[ZAI] MCP(1月):{:.0}%{}",
                color,
                mcp_limit.percentage,
                colors::RESET
            ));
        }

        parts
    }
}

pub fn providers() -> Vec<&'static dyn Provider> {
    static ZHIPU_PROVIDER: ZhipuProvider = ZhipuProvider;
    vec![&ZHIPU_PROVIDER]
}
