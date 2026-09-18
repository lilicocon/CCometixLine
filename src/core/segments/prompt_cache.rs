use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use chrono::Utc;
use std::collections::HashMap;

/// Prompt-cache health from the payload. While the cache is warm: the hit
/// ratio and the time until it expires (`96% · 47m`). Once cold: how many
/// tokens the next request has to write to the cache again (`cold · 75.1k`),
/// which is when a turn costs noticeably more.
#[derive(Default)]
pub struct PromptCacheSegment;

impl PromptCacheSegment {
    pub fn new() -> Self {
        Self
    }

    fn collect_at(&self, input: &InputData, now: i64) -> Option<SegmentData> {
        let cache = input.prompt_cache.as_ref()?;
        // Nothing to report when the provider doesn't report caching at all
        if cache.caching_observed == Some(false) {
            return None;
        }

        // `warm` describes the moment Claude Code built the payload, and the
        // line can stay on screen well past that: re-check the expiry time.
        let seconds_left = cache.expires_at.map(|expires_at| expires_at - now);
        let warm = cache.warm == Some(true) && seconds_left.is_none_or(|secs| secs > 0);

        let (primary, secondary) = if warm {
            let hit_ratio = cache
                .hit_ratio
                .map(|ratio| format!("{:.0}%", ratio * 100.0))
                .unwrap_or_else(|| "warm".to_string());
            let expiry = seconds_left
                .map(|secs| format!("· {}", format_minutes(secs)))
                .unwrap_or_default();
            (hit_ratio, expiry)
        } else {
            let recache = cache
                .recache_tokens_if_cold
                .map(|tokens| format!("· {}", format_tokens(tokens)))
                .unwrap_or_default();
            ("cold".to_string(), recache)
        };

        let mut metadata = HashMap::new();
        metadata.insert("warm".to_string(), warm.to_string());
        if let Some(ratio) = cache.hit_ratio {
            metadata.insert("hit_ratio".to_string(), ratio.to_string());
        }
        if let Some(secs) = seconds_left {
            metadata.insert("expires_in_secs".to_string(), secs.to_string());
        }
        if let Some(tokens) = cache.recache_tokens_if_cold {
            metadata.insert("recache_tokens_if_cold".to_string(), tokens.to_string());
        }
        if let Some(misses) = cache.misses {
            metadata.insert("misses".to_string(), misses.to_string());
        }

        Some(SegmentData {
            primary,
            secondary,
            metadata,
        })
    }
}

impl Segment for PromptCacheSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        self.collect_at(input, Utc::now().timestamp())
    }

    fn id(&self) -> SegmentId {
        SegmentId::PromptCache
    }
}

/// Whole minutes left, or `<1m` in the last minute
fn format_minutes(secs: i64) -> String {
    if secs < 60 {
        "<1m".to_string()
    } else {
        format!("{}m", secs / 60)
    }
}

/// Same style as the context window segment: `75.1k`, `2k`, `850`
fn format_tokens(tokens: u64) -> String {
    if tokens >= 1000 {
        let k_value = tokens as f64 / 1000.0;
        if k_value.fract() == 0.0 {
            format!("{}k", k_value as u64)
        } else {
            format!("{:.1}k", k_value)
        }
    } else {
        tokens.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // expires_at of the captured Claude Code 2.1.275 payload
    const EXPIRES_AT: i64 = 1789699173;

    fn input_with(prompt_cache: serde_json::Value) -> InputData {
        serde_json::from_value(json!({
            "model": {"id": "claude-sonnet-5", "display_name": "Sonnet 5"},
            "workspace": {"current_dir": "/tmp"},
            "transcript_path": "/nonexistent/transcript.jsonl",
            "prompt_cache": prompt_cache,
        }))
        .unwrap()
    }

    fn captured_cache() -> serde_json::Value {
        json!({
            "warm": true,
            "caching_observed": true,
            "ttl": "1h",
            "expires_at": EXPIRES_AT,
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
        })
    }

    #[test]
    fn warm_cache_shows_hit_ratio_and_time_left() {
        let input = input_with(captured_cache());
        let data = PromptCacheSegment::new()
            .collect_at(&input, EXPIRES_AT - 47 * 60 - 30)
            .unwrap();

        assert_eq!(data.primary, "96%");
        assert_eq!(data.secondary, "· 47m");
        assert_eq!(data.metadata["warm"], "true");
    }

    #[test]
    fn warm_cache_in_its_last_minute() {
        let input = input_with(captured_cache());
        let data = PromptCacheSegment::new()
            .collect_at(&input, EXPIRES_AT - 20)
            .unwrap();

        assert_eq!(data.secondary, "· <1m");
    }

    #[test]
    fn expired_cache_is_cold_even_if_the_payload_said_warm() {
        let input = input_with(captured_cache());
        let data = PromptCacheSegment::new()
            .collect_at(&input, EXPIRES_AT + 1)
            .unwrap();

        assert_eq!(data.primary, "cold");
        assert_eq!(data.secondary, "· 75.1k");
        assert_eq!(data.metadata["warm"], "false");
    }

    #[test]
    fn cold_cache_reported_by_claude_code() {
        let mut cache = captured_cache();
        cache["warm"] = json!(false);
        cache["expires_at"] = json!(null);
        cache["recache_tokens_if_cold"] = json!(120000);
        let data = PromptCacheSegment::new()
            .collect_at(&input_with(cache), EXPIRES_AT)
            .unwrap();

        assert_eq!(data.primary, "cold");
        assert_eq!(data.secondary, "· 120k");
    }

    #[test]
    fn hidden_without_cache_data() {
        let mut not_observed = captured_cache();
        not_observed["caching_observed"] = json!(false);

        assert!(PromptCacheSegment::new()
            .collect_at(&input_with(json!(null)), EXPIRES_AT)
            .is_none());
        assert!(PromptCacheSegment::new()
            .collect_at(&input_with(not_observed), EXPIRES_AT)
            .is_none());
    }
}
