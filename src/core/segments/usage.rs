use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use crate::utils::credentials;
use chrono::{DateTime, Datelike, Duration, Local, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct ApiUsageResponse {
    five_hour: UsagePeriod,
    seven_day: UsagePeriod,
}

#[derive(Debug, Deserialize)]
struct UsagePeriod {
    utilization: f64,
    resets_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiUsageCache {
    five_hour_utilization: f64,
    seven_day_utilization: f64,
    resets_at: Option<String>,
    // Absent from caches written before the 5-hour reset time was tracked
    #[serde(default)]
    five_hour_resets_at: Option<String>,
    cached_at: String,
}

/// What the segment displays, whichever source it came from.
struct UsageFigures {
    // Utilization percentages, 0-100
    five_hour: f64,
    seven_day: f64,
    five_hour_resets_at: Option<DateTime<Utc>>,
    seven_day_resets_at: Option<DateTime<Utc>>,
    source: &'static str,
}

#[derive(Default)]
pub struct UsageSegment {
    show_five_hour_reset: bool,
    weekly_split: bool,
}

impl UsageSegment {
    pub fn new() -> Self {
        Self::default()
    }

    /// Also show when the 5-hour window resets, e.g. `5% (14:10)`.
    pub fn with_five_hour_reset(mut self, show: bool) -> Self {
        self.show_five_hour_reset = show;
        self
    }

    /// Leave the 7-day window to the `usage_weekly` segment: the icon then
    /// tracks the 5-hour window and the 7-day reset time is dropped, so this
    /// segment shows the 5-hour window only.
    pub fn with_weekly_split(mut self, split: bool) -> Self {
        self.weekly_split = split;
        self
    }

    /// Payload first; the OAuth usage API (cached) for older Claude Code.
    fn figures(&self, input: &InputData) -> Option<UsageFigures> {
        Self::figures_from_payload(input).or_else(|| self.figures_from_api(input))
    }

    fn get_circle_icon(utilization: f64) -> String {
        let percent = (utilization * 100.0) as u8;
        match percent {
            0..=12 => "\u{f0a9e}".to_string(),  // circle_slice_1
            13..=25 => "\u{f0a9f}".to_string(), // circle_slice_2
            26..=37 => "\u{f0aa0}".to_string(), // circle_slice_3
            38..=50 => "\u{f0aa1}".to_string(), // circle_slice_4
            51..=62 => "\u{f0aa2}".to_string(), // circle_slice_5
            63..=75 => "\u{f0aa3}".to_string(), // circle_slice_6
            76..=87 => "\u{f0aa4}".to_string(), // circle_slice_7
            _ => "\u{f0aa5}".to_string(),       // circle_slice_8
        }
    }

    fn format_reset_time(reset_time: Option<DateTime<Utc>>) -> String {
        if let Some(dt) = reset_time {
            let mut local_dt = dt.with_timezone(&Local);
            if local_dt.minute() > 45 {
                local_dt += Duration::hours(1);
            }
            return format!(
                "{}-{}-{}",
                local_dt.month(),
                local_dt.day(),
                local_dt.hour()
            );
        }
        "?".to_string()
    }

    fn parse_reset_time(rfc3339: Option<&str>) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(rfc3339?)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }

    fn get_cache_path() -> Option<std::path::PathBuf> {
        let home = dirs::home_dir()?;
        Some(
            home.join(".claude")
                .join("ccline")
                .join(".api_usage_cache.json"),
        )
    }

    fn load_cache(&self) -> Option<ApiUsageCache> {
        let cache_path = Self::get_cache_path()?;
        if !cache_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save_cache(&self, cache: &ApiUsageCache) {
        if let Some(cache_path) = Self::get_cache_path() {
            if let Some(parent) = cache_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(cache) {
                let _ = std::fs::write(&cache_path, json);
            }
        }
    }

    fn is_cache_valid(&self, cache: &ApiUsageCache, cache_duration: u64) -> bool {
        if let Ok(cached_at) = DateTime::parse_from_rfc3339(&cache.cached_at) {
            let now = Utc::now();
            let elapsed = now.signed_duration_since(cached_at.with_timezone(&Utc));
            elapsed.num_seconds() < cache_duration as i64
        } else {
            false
        }
    }

    fn get_claude_code_version() -> String {
        use std::process::Command;

        let output = Command::new("npm")
            .args(["view", "@anthropic-ai/claude-code", "version"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !version.is_empty() {
                    return format!("claude-code/{}", version);
                }
            }
            _ => {}
        }

        "claude-code".to_string()
    }

    fn get_proxy_from_settings() -> Option<String> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?;
        let settings_path = format!("{}/.claude/settings.json", home);

        let content = std::fs::read_to_string(&settings_path).ok()?;
        let settings: serde_json::Value = serde_json::from_str(&content).ok()?;

        // Try HTTPS_PROXY first, then HTTP_PROXY
        settings
            .get("env")?
            .get("HTTPS_PROXY")
            .or_else(|| settings.get("env")?.get("HTTP_PROXY"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn fetch_api_usage(
        &self,
        api_base_url: &str,
        token: &str,
        timeout_secs: u64,
        claude_code_version: Option<&str>,
    ) -> Option<ApiUsageResponse> {
        let url = format!("{}/api/oauth/usage", api_base_url);
        // The payload names the running Claude Code; asking npm for the latest
        // published version (a child process plus a network round trip that
        // `timeout` does not cover) is only needed for payloads without it.
        let user_agent = match claude_code_version {
            Some(version) if !version.is_empty() => format!("claude-code/{}", version),
            _ => Self::get_claude_code_version(),
        };

        let agent = if let Some(proxy_url) = Self::get_proxy_from_settings() {
            if let Ok(proxy) = ureq::Proxy::new(&proxy_url) {
                ureq::Agent::config_builder()
                    .proxy(Some(proxy))
                    .build()
                    .new_agent()
            } else {
                ureq::Agent::new_with_defaults()
            }
        } else {
            ureq::Agent::new_with_defaults()
        };

        let response = agent
            .get(&url)
            .header("Authorization", &format!("Bearer {}", token))
            .header("anthropic-beta", "oauth-2025-04-20")
            .header("User-Agent", &user_agent)
            .config()
            .timeout_global(Some(std::time::Duration::from_secs(timeout_secs)))
            .build()
            .call()
            .ok()?;

        response.into_body().read_json().ok()
    }
}

impl UsageSegment {
    /// Usage reported on the payload by Claude Code itself: nothing to
    /// authenticate, fetch or cache, and never stale.
    fn figures_from_payload(input: &InputData) -> Option<UsageFigures> {
        let limits = input.rate_limits.as_ref()?;
        // Claude Code drops a window from the payload once it resets. The
        // display needs both, so until both are back use the API as before.
        let five_hour = limits.five_hour.as_ref()?;
        let seven_day = limits.seven_day.as_ref()?;
        let at = |secs: Option<i64>| secs.and_then(|secs| DateTime::from_timestamp(secs, 0));

        Some(UsageFigures {
            five_hour: five_hour.used_percentage?,
            seven_day: seven_day.used_percentage?,
            five_hour_resets_at: at(five_hour.resets_at),
            seven_day_resets_at: at(seven_day.resets_at),
            source: "payload",
        })
    }

    /// Usage from the OAuth usage API, cached for `cache_duration` seconds:
    /// the source for Claude Code versions that don't send `rate_limits`.
    fn figures_from_api(&self, input: &InputData) -> Option<UsageFigures> {
        let token = credentials::get_oauth_token()?;

        // Load config from file to get segment options
        let config = crate::config::Config::load().ok()?;
        let segment_config = config.segments.iter().find(|s| s.id == SegmentId::Usage);

        let api_base_url = segment_config
            .and_then(|sc| sc.options.get("api_base_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.anthropic.com");

        let cache_duration = segment_config
            .and_then(|sc| sc.options.get("cache_duration"))
            .and_then(|v| v.as_u64())
            .unwrap_or(300);

        let timeout = segment_config
            .and_then(|sc| sc.options.get("timeout"))
            .and_then(|v| v.as_u64())
            .unwrap_or(2);

        let cached_data = self.load_cache();
        let use_cached = cached_data
            .as_ref()
            .map(|cache| self.is_cache_valid(cache, cache_duration))
            .unwrap_or(false);

        let cache = if use_cached {
            cached_data?
        } else {
            match self.fetch_api_usage(api_base_url, &token, timeout, input.version.as_deref()) {
                Some(response) => {
                    let cache = ApiUsageCache {
                        five_hour_utilization: response.five_hour.utilization,
                        seven_day_utilization: response.seven_day.utilization,
                        resets_at: response.seven_day.resets_at,
                        five_hour_resets_at: response.five_hour.resets_at,
                        cached_at: Utc::now().to_rfc3339(),
                    };
                    self.save_cache(&cache);
                    cache
                }
                // Request failed: a stale cache beats showing nothing
                None => cached_data?,
            }
        };

        Some(UsageFigures {
            five_hour: cache.five_hour_utilization,
            seven_day: cache.seven_day_utilization,
            five_hour_resets_at: Self::parse_reset_time(cache.five_hour_resets_at.as_deref()),
            seven_day_resets_at: Self::parse_reset_time(cache.resets_at.as_deref()),
            source: "api",
        })
    }

    fn render(&self, figures: &UsageFigures) -> SegmentData {
        let icon_utilization = if self.weekly_split {
            figures.five_hour
        } else {
            figures.seven_day
        };
        let dynamic_icon = Self::get_circle_icon(icon_utilization / 100.0);
        let five_hour_percent = figures.five_hour.round() as u8;
        let primary = match figures.five_hour_resets_at {
            Some(reset) if self.show_five_hour_reset => format!(
                "{}% ({})",
                five_hour_percent,
                reset.with_timezone(&Local).format("%H:%M")
            ),
            _ => format!("{}%", five_hour_percent),
        };
        let secondary = if self.weekly_split {
            String::new()
        } else {
            format!("· {}", Self::format_reset_time(figures.seven_day_resets_at))
        };

        let mut metadata = HashMap::new();
        metadata.insert("dynamic_icon".to_string(), dynamic_icon);
        metadata.insert(
            "five_hour_utilization".to_string(),
            figures.five_hour.to_string(),
        );
        metadata.insert(
            "seven_day_utilization".to_string(),
            figures.seven_day.to_string(),
        );
        metadata.insert("source".to_string(), figures.source.to_string());

        SegmentData {
            primary,
            secondary,
            metadata,
        }
    }
}

impl Segment for UsageSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let figures = self.figures(input)?;
        Some(self.render(&figures))
    }

    fn id(&self) -> SegmentId {
        SegmentId::Usage
    }
}

/// The 7-day window on its own, next to the `usage` segment and in its own
/// colors: `42% · 09-19` (share used, and the local date it resets). The
/// icon fills with the 7-day share. Enabling it turns `usage` into the
/// 5-hour window only (see `UsageSegment::with_weekly_split`).
#[derive(Default)]
pub struct UsageWeeklySegment;

impl UsageWeeklySegment {
    pub fn new() -> Self {
        Self
    }

    fn render(figures: &UsageFigures) -> SegmentData {
        let secondary = figures
            .seven_day_resets_at
            .map(|reset| format!("· {}", reset.with_timezone(&Local).format("%m-%d")))
            .unwrap_or_default();

        let mut metadata = HashMap::new();
        metadata.insert(
            "dynamic_icon".to_string(),
            UsageSegment::get_circle_icon(figures.seven_day / 100.0),
        );
        metadata.insert(
            "seven_day_utilization".to_string(),
            figures.seven_day.to_string(),
        );
        metadata.insert("source".to_string(), figures.source.to_string());

        SegmentData {
            primary: format!("{}%", figures.seven_day.round() as u8),
            secondary,
            metadata,
        }
    }
}

impl Segment for UsageWeeklySegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        // Same source as `usage`: with both enabled and no payload figures,
        // the second lookup is served by the cache the first one wrote.
        let figures = UsageSegment::new().figures(input)?;
        Some(Self::render(&figures))
    }

    fn id(&self) -> SegmentId {
        SegmentId::UsageWeekly
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input_with(rate_limits: serde_json::Value) -> InputData {
        serde_json::from_value(json!({
            "model": {"id": "claude-sonnet-5", "display_name": "Sonnet 5"},
            "workspace": {"current_dir": "/tmp"},
            "transcript_path": "/nonexistent/transcript.jsonl",
            "version": "2.1.275",
            "rate_limits": rate_limits,
        }))
        .unwrap()
    }

    fn captured_limits() -> serde_json::Value {
        json!({
            "five_hour": {"used_percentage": 5, "resets_at": 1789697400},
            "seven_day": {"used_percentage": 53, "resets_at": 1789826400}
        })
    }

    fn local(epoch: i64) -> DateTime<Local> {
        DateTime::from_timestamp(epoch, 0)
            .unwrap()
            .with_timezone(&Local)
    }

    #[test]
    fn renders_payload_rate_limits_without_the_api() {
        let input = input_with(captured_limits());
        // collect() must not need credentials, config or network for this
        let data = UsageSegment::new().collect(&input).unwrap();

        assert_eq!(data.primary, "5%");
        assert_eq!(
            data.secondary,
            format!(
                "· {}",
                UsageSegment::format_reset_time(DateTime::from_timestamp(1789826400, 0))
            )
        );
        assert_eq!(data.metadata["source"], "payload");
        assert_eq!(data.metadata["seven_day_utilization"], "53");
        // 53% of the week used -> circle_slice_5
        assert_eq!(data.metadata["dynamic_icon"], "\u{f0aa2}");
    }

    #[test]
    fn five_hour_reset_time_is_opt_in() {
        let input = input_with(captured_limits());
        let data = UsageSegment::new()
            .with_five_hour_reset(true)
            .collect(&input)
            .unwrap();

        assert_eq!(
            data.primary,
            format!("5% ({})", local(1789697400).format("%H:%M"))
        );
    }

    #[test]
    fn needs_both_windows_to_skip_the_api() {
        let only_five_hour = input_with(json!({
            "five_hour": {"used_percentage": 5, "resets_at": 1789697400}
        }));
        let no_percentage = input_with(json!({
            "five_hour": {"resets_at": 1789697400},
            "seven_day": {"used_percentage": 53, "resets_at": 1789826400}
        }));
        let spend_limit_only = input_with(json!({
            "spend_limit": {"used_percentage": 12, "resets_at": 1789826400}
        }));

        assert!(UsageSegment::figures_from_payload(&only_five_hour).is_none());
        assert!(UsageSegment::figures_from_payload(&no_percentage).is_none());
        assert!(UsageSegment::figures_from_payload(&spend_limit_only).is_none());
        assert!(UsageSegment::figures_from_payload(&input_with(json!(null))).is_none());
    }

    #[test]
    fn reads_cache_files_written_by_older_versions() {
        let cache: ApiUsageCache = serde_json::from_str(
            r#"{"five_hour_utilization": 5.0, "seven_day_utilization": 53.0,
                "resets_at": "2026-09-25T02:00:00+00:00",
                "cached_at": "2026-09-18T01:48:00+00:00"}"#,
        )
        .unwrap();

        assert!(cache.five_hour_resets_at.is_none());
        assert_eq!(
            UsageSegment::parse_reset_time(cache.resets_at.as_deref()),
            DateTime::from_timestamp(1790301600, 0)
        );
    }

    #[test]
    fn weekly_split_leaves_usage_with_the_five_hour_window() {
        let input = input_with(captured_limits());
        let data = UsageSegment::new()
            .with_weekly_split(true)
            .collect(&input)
            .unwrap();

        assert_eq!(data.primary, "5%");
        // The 7-day reset moves to usage_weekly
        assert_eq!(data.secondary, "");
        // 5% of the 5-hour window used -> circle_slice_1, not the week's 53%
        assert_eq!(data.metadata["dynamic_icon"], "\u{f0a9e}");
    }

    #[test]
    fn usage_weekly_shows_the_seven_day_window() {
        let input = input_with(captured_limits());
        let data = UsageWeeklySegment::new().collect(&input).unwrap();

        assert_eq!(data.primary, "53%");
        assert_eq!(
            data.secondary,
            format!("· {}", local(1789826400).format("%m-%d"))
        );
        // Zero-padded month and day, e.g. "09-19"
        assert_eq!(data.secondary.len(), "· 09-19".len());
        // 53% of the week used -> circle_slice_5
        assert_eq!(data.metadata["dynamic_icon"], "\u{f0aa2}");
        assert_eq!(data.metadata["source"], "payload");
    }

    #[test]
    fn usage_weekly_without_a_reset_time_shows_the_percentage_only() {
        let input = input_with(json!({
            "five_hour": {"used_percentage": 5},
            "seven_day": {"used_percentage": 53}
        }));
        let data = UsageWeeklySegment::new().collect(&input).unwrap();

        assert_eq!(data.primary, "53%");
        assert_eq!(data.secondary, "");
    }
}
