use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

// Main config structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub style: StyleConfig,
    pub segments: Vec<SegmentConfig>,
    pub theme: String,
}

// Default implementation moved to ui/themes/presets.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConfig {
    pub mode: StyleMode,
    pub separator: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StyleMode {
    Plain,
    NerdFont,
    Powerline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentConfig {
    pub id: SegmentId,
    pub enabled: bool,
    pub icon: IconConfig,
    pub colors: ColorConfig,
    pub styles: TextStyleConfig,
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconConfig {
    pub plain: String,
    pub nerd_font: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    pub icon: Option<AnsiColor>,
    pub text: Option<AnsiColor>,
    pub background: Option<AnsiColor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextStyleConfig {
    pub text_bold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnsiColor {
    Color16 { c16: u8 },
    Color256 { c256: u8 },
    Rgb { r: u8, g: u8, b: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentId {
    Model,
    Directory,
    Git,
    ContextWindow,
    Usage,
    Cost,
    Session,
    OutputStyle,
    Update,
    PromptCache,
    Mode,
    UsageWeekly,
}

// Legacy compatibility structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SegmentsConfig {
    pub directory: bool,
    pub git: bool,
    pub model: bool,
    // pub usage: bool,
}

// Data structures compatible with existing main.rs
#[derive(Deserialize)]
pub struct Model {
    pub id: String,
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct Workspace {
    pub current_dir: String,
    #[serde(default, deserialize_with = "lenient")]
    pub project_dir: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub added_dirs: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct Cost {
    pub total_cost_usd: Option<f64>,
    pub total_duration_ms: Option<u64>,
    pub total_api_duration_ms: Option<u64>,
    pub total_lines_added: Option<u32>,
    pub total_lines_removed: Option<u32>,
}

#[derive(Deserialize)]
pub struct OutputStyle {
    pub name: String,
}

#[derive(Deserialize)]
pub struct InputData {
    pub model: Model,
    pub workspace: Workspace,
    pub transcript_path: String,
    pub cost: Option<Cost>,
    pub output_style: Option<OutputStyle>,

    // Fields added by newer Claude Code releases. Each is optional and parsed
    // leniently: absent (older Claude Code) or malformed (a future schema
    // change) values become `None` instead of failing the whole status line.
    #[serde(default, deserialize_with = "lenient")]
    pub context_window: Option<ContextWindowInfo>,
    #[serde(default, deserialize_with = "lenient")]
    pub exceeds_200k_tokens: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    pub rate_limits: Option<RateLimits>,
    #[serde(default, deserialize_with = "lenient")]
    pub prompt_cache: Option<PromptCache>,
    #[serde(default, deserialize_with = "lenient")]
    pub effort: Option<Effort>,
    #[serde(default, deserialize_with = "lenient")]
    pub thinking: Option<Thinking>,
    #[serde(default, deserialize_with = "lenient")]
    pub fast_mode: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    pub version: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub session_id: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub session_name: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub prompt_id: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub cwd: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub scratchpad_dir: Option<String>,
}

/// Deserialize an optional payload field without letting it fail the whole
/// payload: a missing, `null` or malformed value all become `None`. Fields
/// using it also need `#[serde(default)]`: serde only fills in `None` for an
/// absent `Option` field by itself when no `deserialize_with` is set.
fn lenient<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).ok())
}

// Authoritative context-window usage reported directly by Claude Code.
// Present since Claude Code added support for extended context windows
// (e.g. the 1M-token beta), where the true window size can't be inferred
// from the model id alone.
#[derive(Deserialize)]
pub struct ContextWindowInfo {
    pub total_input_tokens: Option<u32>,
    pub total_output_tokens: Option<u32>,
    pub context_window_size: Option<u32>,
    // Usage of the most recent API call (same shape as a transcript `usage`
    // object); null until the session's first response.
    pub current_usage: Option<Usage>,
    // Claude Code's own rounded figure: input side only, 0-100.
    pub used_percentage: Option<f64>,
    pub remaining_percentage: Option<f64>,
}

// Claude.ai subscription limits (or a Claude gateway spend limit) as Claude
// Code tracks them from API responses. Only sent after the first response,
// and each window is dropped once its `resets_at` has passed.
#[derive(Deserialize)]
pub struct RateLimits {
    pub five_hour: Option<RateLimitWindow>,
    pub seven_day: Option<RateLimitWindow>,
    pub spend_limit: Option<RateLimitWindow>,
}

#[derive(Deserialize)]
pub struct RateLimitWindow {
    // 0-100 (a spend limit can go above 100 once exceeded)
    pub used_percentage: Option<f64>,
    // Unix epoch seconds
    pub resets_at: Option<i64>,
}

// Prompt-cache health of the main conversation, sent after the first
// API response. Timestamps are Unix epoch seconds.
#[derive(Deserialize)]
pub struct PromptCache {
    // Cached prefix still inside its TTL when the payload was built
    pub warm: Option<bool>,
    // Whether any response reported cache tokens at all
    pub caching_observed: Option<bool>,
    // "5m" | "1h"
    pub ttl: Option<String>,
    pub expires_at: Option<i64>,
    pub requests: Option<u64>,
    pub misses: Option<u64>,
    pub expected_rebuilds: Option<u64>,
    // cache_read / (cache_read + cache_creation + uncached input), 0-1
    pub hit_ratio: Option<f64>,
    pub cache_write_tokens: Option<u64>,
    pub miss_recache_tokens: Option<u64>,
    pub last_miss_at: Option<i64>,
    pub last_miss_cause: Option<PromptCacheMissCause>,
    pub miss_causes: Option<HashMap<String, u64>>,
    // Tokens the next request re-caches if the cache is cold by then
    pub recache_tokens_if_cold: Option<u64>,
}

#[derive(Deserialize)]
pub struct PromptCacheMissCause {
    pub causes: Option<Vec<String>>,
    pub tools_added: Option<u64>,
    pub tools_removed: Option<u64>,
    pub system_char_delta: Option<i64>,
}

// Only sent when the current model supports reasoning effort.
#[derive(Deserialize)]
pub struct Effort {
    // "low" | "medium" | "high" | "xhigh" | "max"
    pub level: Option<String>,
}

#[derive(Deserialize)]
pub struct Thinking {
    pub enabled: Option<bool>,
}

// OpenAI-style nested token details
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PromptTokensDetails {
    #[serde(default)]
    pub cached_tokens: Option<u32>,
    #[serde(default)]
    pub audio_tokens: Option<u32>,
}

// Raw usage data from different LLM providers (flexible parsing)
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RawUsage {
    // Anthropic-style input tokens
    #[serde(default)]
    pub input_tokens: Option<u32>,

    // OpenAI-style input tokens (separate field to handle both formats)
    #[serde(default)]
    pub prompt_tokens: Option<u32>,

    // Anthropic-style output tokens
    #[serde(default)]
    pub output_tokens: Option<u32>,

    // OpenAI-style output tokens (separate field to handle both formats)
    #[serde(default)]
    pub completion_tokens: Option<u32>,

    // Total tokens (some providers only provide this)
    #[serde(default)]
    pub total_tokens: Option<u32>,

    // Anthropic-style cache fields
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u32>,

    #[serde(default)]
    pub cache_read_input_tokens: Option<u32>,

    // OpenAI-style cache fields (separate fields to handle both formats)
    #[serde(default)]
    pub cache_creation_prompt_tokens: Option<u32>,

    #[serde(default)]
    pub cache_read_prompt_tokens: Option<u32>,

    #[serde(default)]
    pub cached_tokens: Option<u32>,

    // OpenAI-style nested details
    #[serde(default)]
    pub prompt_tokens_details: Option<PromptTokensDetails>,

    // Completion token details (OpenAI)
    #[serde(default)]
    pub completion_tokens_details: Option<HashMap<String, u32>>,

    // Catch unknown fields for future compatibility and debugging
    #[serde(flatten, skip_serializing)]
    pub extra: HashMap<String, serde_json::Value>,
}

// Normalized internal representation after processing
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct NormalizedUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,

    // Metadata for debugging and analysis
    pub calculation_source: String,
    pub raw_data_available: Vec<String>,
}

impl NormalizedUsage {
    /// Get tokens that count toward context window
    /// This includes all tokens that consume context window space
    /// Output tokens from this turn will become input tokens in the next turn
    pub fn context_tokens(&self) -> u32 {
        self.input_tokens
            + self.cache_creation_input_tokens
            + self.cache_read_input_tokens
            + self.output_tokens
    }

    /// Get total tokens for cost calculation
    /// Priority: use total_tokens if available, otherwise sum all components
    pub fn total_for_cost(&self) -> u32 {
        if self.total_tokens > 0 {
            self.total_tokens
        } else {
            self.input_tokens
                + self.output_tokens
                + self.cache_creation_input_tokens
                + self.cache_read_input_tokens
        }
    }

    /// Get the most appropriate token count for general display
    /// For OpenAI format: use total_tokens directly
    /// For Anthropic format: use context_tokens (input + cache)
    pub fn display_tokens(&self) -> u32 {
        // For Claude/Anthropic format: prefer input-related tokens for context window display
        let context = self.context_tokens();
        if context > 0 {
            return context;
        }

        // For OpenAI format: use total_tokens when no input breakdown available
        if self.total_tokens > 0 {
            return self.total_tokens;
        }

        // Fallback to any available tokens
        self.input_tokens.max(self.output_tokens)
    }
}

impl Config {
    /// Check if current config matches the specified theme preset
    pub fn matches_theme(&self, theme_name: &str) -> bool {
        let theme_preset = crate::ui::themes::ThemePresets::get_theme(theme_name);

        // Compare style config
        if self.style.mode != theme_preset.style.mode
            || self.style.separator != theme_preset.style.separator
        {
            return false;
        }

        // Compare segments count and order
        if self.segments.len() != theme_preset.segments.len() {
            return false;
        }

        // Compare each segment config
        for (current, preset) in self.segments.iter().zip(theme_preset.segments.iter()) {
            if !self.segment_matches(current, preset) {
                return false;
            }
        }

        true
    }

    /// Check if current config has been modified from the selected theme
    pub fn is_modified_from_theme(&self) -> bool {
        !self.matches_theme(&self.theme)
    }

    /// Compare two segment configs for equality
    fn segment_matches(&self, current: &SegmentConfig, preset: &SegmentConfig) -> bool {
        current.id == preset.id
            && current.enabled == preset.enabled
            && current.icon.plain == preset.icon.plain
            && current.icon.nerd_font == preset.icon.nerd_font
            && self.color_matches(&current.colors.icon, &preset.colors.icon)
            && self.color_matches(&current.colors.text, &preset.colors.text)
            && self.color_matches(&current.colors.background, &preset.colors.background)
            && current.styles.text_bold == preset.styles.text_bold
            && current.options == preset.options
    }

    /// Compare two optional colors for equality
    fn color_matches(&self, current: &Option<AnsiColor>, preset: &Option<AnsiColor>) -> bool {
        match (current, preset) {
            (None, None) => true,
            (Some(c1), Some(c2)) => c1 == c2,
            _ => false,
        }
    }
}

impl PartialEq for AnsiColor {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AnsiColor::Color16 { c16: a }, AnsiColor::Color16 { c16: b }) => a == b,
            (AnsiColor::Color256 { c256: a }, AnsiColor::Color256 { c256: b }) => a == b,
            (
                AnsiColor::Rgb {
                    r: r1,
                    g: g1,
                    b: b1,
                },
                AnsiColor::Rgb {
                    r: r2,
                    g: g2,
                    b: b2,
                },
            ) => r1 == r2 && g1 == g2 && b1 == b2,
            _ => false,
        }
    }
}

impl RawUsage {
    /// Convert raw usage data to normalized format with intelligent token inference
    pub fn normalize(self) -> NormalizedUsage {
        let mut result = NormalizedUsage::default();
        let mut sources = Vec::new();

        // Collect available raw data fields and merge tokens with Anthropic priority
        let mut available_fields = Vec::new();

        // Merge input tokens (priority: input_tokens > prompt_tokens)
        let input = self.input_tokens.or(self.prompt_tokens).unwrap_or(0);
        if input > 0 {
            available_fields.push("input_tokens".to_string());
        }

        // Merge output tokens (priority: output_tokens > completion_tokens)
        let output = self.output_tokens.or(self.completion_tokens).unwrap_or(0);
        if output > 0 {
            available_fields.push("output_tokens".to_string());
        }

        let total = self.total_tokens.unwrap_or(0);
        if total > 0 {
            available_fields.push("total_tokens".to_string());
        }

        // Merge cache creation tokens (priority: Anthropic > OpenAI)
        let cache_creation = self
            .cache_creation_input_tokens
            .or(self.cache_creation_prompt_tokens)
            .unwrap_or(0);
        if cache_creation > 0 {
            available_fields.push("cache_creation".to_string());
        }

        // Merge cache read tokens (priority: Anthropic > OpenAI > nested format)
        let cache_read = self
            .cache_read_input_tokens
            .or(self.cache_read_prompt_tokens)
            .or(self.cached_tokens)
            .or_else(|| {
                // Fallback to OpenAI nested format
                self.prompt_tokens_details
                    .as_ref()
                    .and_then(|d| d.cached_tokens)
            })
            .unwrap_or(0);
        if cache_read > 0 {
            available_fields.push("cache_read".to_string());
        }

        result.raw_data_available = available_fields;

        // Use merged cache values (already calculated above with Anthropic priority)

        // Token calculation logic - prioritize total_tokens for OpenAI format
        let total_value = if total > 0 {
            sources.push("total_tokens_direct".to_string());
            total
        } else if input > 0 || output > 0 || cache_read > 0 || cache_creation > 0 {
            let calculated = input + output + cache_read + cache_creation;
            sources.push("total_from_components".to_string());
            calculated
        } else {
            0
        };

        // Assignment
        result.input_tokens = input;
        result.output_tokens = output;
        result.total_tokens = total_value;
        result.cache_creation_input_tokens = cache_creation;
        result.cache_read_input_tokens = cache_read;
        result.calculation_source = sources.join("+");

        result
    }
}

// Legacy alias for backward compatibility
pub type Usage = RawUsage;

#[derive(Deserialize)]
pub struct Message {
    pub usage: Option<Usage>,
}

#[derive(Deserialize)]
pub struct TranscriptEntry {
    pub r#type: Option<String>,
    pub message: Option<Message>,
    #[serde(rename = "leafUuid")]
    pub leaf_uuid: Option<String>,
    pub uuid: Option<String>,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    pub summary: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// statusLine payload captured from Claude Code 2.1.275 (Sonnet 5, 1M context).
    fn captured_payload() -> serde_json::Value {
        json!({
            "session_id": "0c4e2c1e-6a53-4a8e-9a62-7d0f3f6b0001",
            "transcript_path": "/tmp/session.jsonl",
            "cwd": "/work/repo",
            "scratchpad_dir": "/tmp/scratchpad",
            "prompt_id": "5d1d2d1e-0000-4000-8000-000000000001",
            "effort": {"level": "high"},
            "session_name": "ccline upgrade",
            "model": {"id": "claude-sonnet-5", "display_name": "Sonnet 5"},
            "workspace": {"current_dir": "/work/repo", "project_dir": "/work/repo", "added_dirs": []},
            "version": "2.1.275",
            "output_style": {"name": "default"},
            "cost": {
                "total_cost_usd": 0.426719,
                "total_duration_ms": 1341239,
                "total_api_duration_ms": 115428,
                "total_lines_added": 4,
                "total_lines_removed": 1
            },
            "context_window": {
                "total_input_tokens": 75086,
                "total_output_tokens": 144,
                "context_window_size": 1000000,
                "current_usage": {
                    "input_tokens": 2,
                    "output_tokens": 144,
                    "cache_creation_input_tokens": 896,
                    "cache_read_input_tokens": 74188
                },
                "used_percentage": 8,
                "remaining_percentage": 92
            },
            "exceeds_200k_tokens": false,
            "prompt_cache": {
                "warm": true,
                "caching_observed": true,
                "ttl": "1h",
                "expires_at": 1789699173,
                "requests": 13,
                "misses": 0,
                "expected_rebuilds": 0,
                "hit_ratio": 0.9628340706473149,
                "cache_write_tokens": 33604,
                "miss_recache_tokens": 0,
                "last_miss_at": null,
                "last_miss_cause": null,
                "miss_causes": {},
                "recache_tokens_if_cold": 75086
            },
            "fast_mode": false,
            "thinking": {"enabled": true},
            "rate_limits": {
                "five_hour": {"used_percentage": 5, "resets_at": 1789697400},
                "seven_day": {"used_percentage": 53, "resets_at": 1789826400}
            }
        })
    }

    // main.rs parses from a reader, so go through text rather than from_value
    fn parse(payload: serde_json::Value) -> InputData {
        serde_json::from_str(&payload.to_string()).unwrap()
    }

    #[test]
    fn parses_every_field_of_a_current_payload() {
        let input = parse(captured_payload());

        assert_eq!(
            input.session_id.as_deref(),
            Some("0c4e2c1e-6a53-4a8e-9a62-7d0f3f6b0001")
        );
        assert_eq!(input.session_name.as_deref(), Some("ccline upgrade"));
        assert_eq!(
            input.prompt_id.as_deref(),
            Some("5d1d2d1e-0000-4000-8000-000000000001")
        );
        assert_eq!(input.cwd.as_deref(), Some("/work/repo"));
        assert_eq!(input.scratchpad_dir.as_deref(), Some("/tmp/scratchpad"));
        assert_eq!(input.version.as_deref(), Some("2.1.275"));
        assert_eq!(input.workspace.project_dir.as_deref(), Some("/work/repo"));
        assert_eq!(input.workspace.added_dirs, Some(vec![]));
        assert_eq!(input.exceeds_200k_tokens, Some(false));
        assert_eq!(input.fast_mode, Some(false));
        assert_eq!(input.thinking.unwrap().enabled, Some(true));
        assert_eq!(input.effort.unwrap().level.as_deref(), Some("high"));

        let context = input.context_window.unwrap();
        assert_eq!(context.context_window_size, Some(1_000_000));
        assert_eq!(context.total_input_tokens, Some(75086));
        assert_eq!(
            context.current_usage.unwrap().cache_read_input_tokens,
            Some(74188)
        );
        assert_eq!(context.used_percentage, Some(8.0));
        assert_eq!(context.remaining_percentage, Some(92.0));

        let limits = input.rate_limits.unwrap();
        let five_hour = limits.five_hour.unwrap();
        assert_eq!(five_hour.used_percentage, Some(5.0));
        assert_eq!(five_hour.resets_at, Some(1789697400));
        assert_eq!(limits.seven_day.unwrap().used_percentage, Some(53.0));
        assert!(limits.spend_limit.is_none());

        let cache = input.prompt_cache.unwrap();
        assert_eq!(cache.warm, Some(true));
        assert_eq!(cache.caching_observed, Some(true));
        assert_eq!(cache.ttl.as_deref(), Some("1h"));
        assert_eq!(cache.expires_at, Some(1789699173));
        assert_eq!(cache.requests, Some(13));
        assert_eq!(cache.misses, Some(0));
        assert_eq!(cache.expected_rebuilds, Some(0));
        assert_eq!(cache.hit_ratio, Some(0.9628340706473149));
        assert_eq!(cache.cache_write_tokens, Some(33604));
        assert_eq!(cache.miss_recache_tokens, Some(0));
        assert_eq!(cache.last_miss_at, None);
        assert!(cache.last_miss_cause.is_none());
        assert_eq!(cache.miss_causes, Some(HashMap::new()));
        assert_eq!(cache.recache_tokens_if_cold, Some(75086));
    }

    #[test]
    fn parses_a_payload_from_older_claude_code() {
        let input = parse(json!({
            "model": {"id": "claude-sonnet-4-20250514", "display_name": "Sonnet 4"},
            "workspace": {"current_dir": "/work/repo"},
            "transcript_path": "/tmp/session.jsonl",
            "cost": {"total_cost_usd": 0.01},
            "output_style": {"name": "default"}
        }));

        assert!(input.context_window.is_none());
        assert!(input.rate_limits.is_none());
        assert!(input.prompt_cache.is_none());
        assert!(input.effort.is_none());
        assert!(input.thinking.is_none());
        assert!(input.fast_mode.is_none());
        assert!(input.exceeds_200k_tokens.is_none());
        assert!(input.version.is_none());
        assert!(input.session_id.is_none());
        assert!(input.workspace.project_dir.is_none());
        assert!(input.workspace.added_dirs.is_none());
    }

    #[test]
    fn a_malformed_new_field_degrades_to_none_instead_of_failing() {
        let mut payload = captured_payload();
        payload["effort"] = json!("high");
        payload["rate_limits"]["five_hour"]["resets_at"] = json!("soon");
        payload["workspace"]["added_dirs"] = json!("/work/other");
        payload["fast_mode"] = json!(null);

        let input = parse(payload);

        assert!(input.effort.is_none());
        assert!(input.rate_limits.is_none());
        assert!(input.workspace.added_dirs.is_none());
        assert!(input.fast_mode.is_none());
        // Everything else is unaffected
        assert_eq!(input.workspace.current_dir, "/work/repo");
        assert_eq!(input.prompt_cache.unwrap().requests, Some(13));
        assert_eq!(
            input.context_window.unwrap().context_window_size,
            Some(1_000_000)
        );
    }
}
