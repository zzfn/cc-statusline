use chrono::{DateTime, Datelike, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
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
        if age.num_minutes() < 5 {
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

// ════════════════════════════════════════════════════════════════════════════════
// Anthropic 官方 API 用量
// ════════════════════════════════════════════════════════════════════════════════

/// Anthropic 官方用量响应
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnthropicUsage {
    pub five_hour: AnthropicPeriod,
    pub seven_day: AnthropicPeriod,
    #[serde(default)]
    pub extra_usage: Option<AnthropicExtra>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnthropicPeriod {
    pub utilization: f64,
    pub resets_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnthropicExtra {
    pub is_enabled: bool,
    #[serde(default)]
    pub used_credits: Option<f64>,
    #[serde(default)]
    pub monthly_limit: Option<u64>,
}

/// Anthropic 用量缓存
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnthropicUsageCache {
    pub usage: AnthropicUsage,
    pub timestamp: DateTime<Utc>,
}

pub struct AnthropicOfficial;

impl AnthropicOfficial {
    fn cache_path(&self) -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".claude")
            .join(".anthropic_usage_cache.json")
    }

    fn lock_path(&self) -> PathBuf {
        self.cache_path().with_extension("lock")
    }

    fn read_cache(&self) -> Option<AnthropicUsageCache> {
        let cache_path = self.cache_path();
        let content = fs::read_to_string(cache_path).ok()?;
        let cache: AnthropicUsageCache = serde_json::from_str(&content).ok()?;

        // 缓存 5 分钟
        let now = Utc::now();
        let age = now.signed_duration_since(cache.timestamp);
        if age.num_minutes() < 5 {
            Some(cache)
        } else {
            None
        }
    }

    /// 读取旧缓存（不检查过期），用于锁竞争时降级返回
    fn read_stale_cache(&self) -> Option<AnthropicUsageCache> {
        let content = fs::read_to_string(self.cache_path()).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn write_cache(&self, cache: &AnthropicUsageCache) {
        let cache_path = self.cache_path();
        if let Ok(json) = serde_json::to_string(cache) {
            let _ = fs::write(cache_path, json);
        }
    }

    /// 获取 OAuth token
    fn get_oauth_token(&self) -> Option<String> {
        // 1. 环境变量
        if let Ok(token) = std::env::var("CLAUDE_CODE_OAUTH_TOKEN") {
            if !token.is_empty() {
                return Some(token);
            }
        }

        // 2. macOS Keychain
        if cfg!(target_os = "macos") {
            let output = Command::new("security")
                .args(&["find-generic-password", "-s", "Claude Code-credentials", "-w"])
                .output()
                .ok()?;

            if output.status.success() {
                let blob = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&blob) {
                    if let Some(token) = json["claudeAiOauth"]["accessToken"].as_str() {
                        return Some(token.to_string());
                    }
                }
            }
        }

        // 3. 凭证文件
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?;
        let creds_path = PathBuf::from(home).join(".claude").join(".credentials.json");

        if creds_path.exists() {
            if let Ok(content) = fs::read_to_string(&creds_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(token) = json["claudeAiOauth"]["accessToken"].as_str() {
                        return Some(token.to_string());
                    }
                }
            }
        }

        // 4. Linux secret-tool
        if cfg!(target_os = "linux") {
            let output = Command::new("timeout")
                .args(&["2", "secret-tool", "lookup", "service", "Claude Code-credentials"])
                .output()
                .ok()?;

            if output.status.success() {
                let blob = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&blob) {
                    if let Some(token) = json["claudeAiOauth"]["accessToken"].as_str() {
                        return Some(token.to_string());
                    }
                }
            }
        }

        None
    }

    fn fetch_usage(&self) -> Option<AnthropicUsageCache> {
        let token = self.get_oauth_token()?;

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;

        let response = client
            .get("https://claude.ai/api/oauth/usage")
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", token))
            .header("anthropic-beta", "oauth-2025-04-20")
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15")
            .send()
            .ok()?;

        if !response.status().is_success() {
            return None;
        }

        let usage: AnthropicUsage = response.json().ok()?;

        let cache = AnthropicUsageCache {
            usage,
            timestamp: Utc::now(),
        };

        self.write_cache(&cache);
        Some(cache)
    }

    fn get_usage(&self) -> Option<AnthropicUsageCache> {
        if let Some(cache) = self.read_cache() {
            return Some(cache);
        }

        // 用 create_new 原子抢锁，失败说明其他进程正在请求，降级返回旧缓存
        let lock_path = self.lock_path();
        let lock = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path);

        if lock.is_err() {
            return self.read_stale_cache();
        }

        let result = self.fetch_usage();
        let _ = fs::remove_file(&lock_path);
        result
    }

    /// 格式化重置时间
    fn format_reset_time(iso_str: &str) -> Option<String> {
        // 尝试解析 ISO 8601 时间
        let dt = DateTime::parse_from_rfc3339(iso_str).ok()?;
        let dt_local = dt.with_timezone(&chrono::Local);

        let now = Utc::now();
        let dt_utc = DateTime::parse_from_rfc3339(iso_str).ok()?.with_timezone(&Utc);
        let is_today = (dt_utc - now).num_hours() < 24;

        if is_today {
            // 只显示时间，如 "10:30pm"
            Some(dt_local.format("%-l:%M%P").to_string().replace("  ", " "))
        } else {
            // 显示日期和时间，如 "mar 16, 10:30pm"
            Some(dt_local.format("%b %-d, %-l:%M%P").to_string().replace("  ", " "))
        }
    }

}

impl Provider for AnthropicOfficial {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn matches(&self, base_url: &str) -> bool {
        // 匹配官方 API 或没有配置第三方 base_url
        base_url.contains("api.anthropic.com") || base_url.is_empty()
    }

    fn get_parts(&self, _base_url: &str, _auth_token: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let Some(cache) = self.get_usage() else {
            return parts;
        };

        let usage = &cache.usage;

        // 五小时用量
        let five_hr_pct = usage.five_hour.utilization;
        let five_hr_reset = usage
            .five_hour
            .resets_at
            .as_ref()
            .and_then(|r| Self::format_reset_time(r))
            .unwrap_or_else(|| "?".to_string());

        parts.push(format!(
            "{}current{} {:3.0}%{} {} {}{}",
            colors::WHITE,
            colors::RESET,
            five_hr_pct,
            colors::RESET,
            colors::DIM,
            five_hr_reset,
            colors::RESET
        ));

        // 七天用量
        let seven_day_pct = usage.seven_day.utilization;
        let seven_day_reset = usage
            .seven_day
            .resets_at
            .as_ref()
            .and_then(|r| Self::format_reset_time(r))
            .unwrap_or_else(|| "?".to_string());

        parts.push(format!(
            "{}weekly{}  {:3.0}%{} {} {}{}",
            colors::WHITE,
            colors::RESET,
            seven_day_pct,
            colors::RESET,
            colors::DIM,
            seven_day_reset,
            colors::RESET
        ));

        // 额外用量（如果启用）
        if let Some(ref extra) = usage.extra_usage {
            if extra.is_enabled {
                let used = extra.used_credits.unwrap_or(0.0);
                let limit = extra.monthly_limit.unwrap_or(0) as f64 / 100.0;
                let _extra_pct = if limit > 0.0 {
                    (used / limit) * 100.0
                } else {
                    0.0
                };
                // 下个月 1 号
                let now = chrono::Local::now();
                let next_month = if now.month() == 12 {
                    now.with_year(now.year() + 1)
                        .and_then(|d| d.with_month(1))
                } else {
                    now.with_month(now.month() + 1)
                };
                let extra_reset = next_month
                    .map(|d| d.format("%b %-d").to_string())
                    .unwrap_or_else(|| "?".to_string());

                parts.push(format!(
                    "{}extra{}   ${:.2}/{:.2} {}{}{}",
                    colors::WHITE,
                    colors::RESET,
                    used,
                    limit,
                    colors::DIM,
                    extra_reset,
                    colors::RESET
                ));
            }
        }

        parts
    }
}

pub fn providers() -> Vec<&'static dyn Provider> {
    static ZHIPU_PROVIDER: ZhipuProvider = ZhipuProvider;
    static ANTHROPIC_PROVIDER: AnthropicOfficial = AnthropicOfficial;
    vec![&ZHIPU_PROVIDER, &ANTHROPIC_PROVIDER]
}

// ════════════════════════════════════════════════════════════════════════════════
// Claude 双倍用量状态（isclaude2x.com）
// ════════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Claude2xData {
    pub is2x: bool,
    #[serde(rename = "isPeak")]
    pub is_peak: bool,
    #[serde(rename = "2xWindowExpiresIn")]
    pub x2_window_expires_in: Option<String>,
    #[serde(rename = "standardWindowExpiresIn")]
    pub standard_window_expires_in: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Claude2xCache {
    pub data: Claude2xData,
    pub timestamp: DateTime<Utc>,
}

fn claude2x_cache_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude").join(".claude2x_cache.json")
}

fn read_claude2x_cache() -> Option<Claude2xCache> {
    let content = fs::read_to_string(claude2x_cache_path()).ok()?;
    let cache: Claude2xCache = serde_json::from_str(&content).ok()?;
    // 缓存 5 分钟
    if Utc::now().signed_duration_since(cache.timestamp).num_minutes() < 5 {
        Some(cache)
    } else {
        None
    }
}

fn write_claude2x_cache(cache: &Claude2xCache) {
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = fs::write(claude2x_cache_path(), json);
    }
}

fn fetch_claude2x() -> Option<Claude2xCache> {
    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .ok()?;

    let response = client
        .get("https://isclaude2x.com/json")
        .header("User-Agent", "claude-code/statusline")
        .send()
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let data: Claude2xData = response.json().ok()?;
    let cache = Claude2xCache { data, timestamp: Utc::now() };
    write_claude2x_cache(&cache);
    Some(cache)
}

/// 去掉秒数，只保留小时/分钟部分，例如 "2h 25m 21s" → "2h 25m"
fn trim_seconds(s: &str) -> &str {
    // 找最后一个包含 'm' 的位置截断
    if let Some(pos) = s.find('m') {
        // 如果 m 后面还有内容（如 " 21s"），截掉
        let after = s[pos + 1..].trim_start();
        if after.ends_with('s') || after.contains('s') {
            return s[..pos + 1].trim_end();
        }
    }
    s
}

/// 获取 2x 状态的 statusline 段落，仅在活跃时返回 Some
pub fn get_claude_2x_part() -> Option<String> {
    let cache = read_claude2x_cache().or_else(fetch_claude2x)?;
    let data = &cache.data;

    if data.is2x {
        let expires = data
            .x2_window_expires_in
            .as_deref()
            .map(trim_seconds)
            .unwrap_or("?");
        Some(format!(
            "{}{}2x⚡({}){}",
            colors::BOLD,
            colors::GREEN,
            expires,
            colors::RESET
        ))
    } else {
        None
    }
}
