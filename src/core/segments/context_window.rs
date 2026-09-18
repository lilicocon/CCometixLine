use super::{Segment, SegmentData};
use crate::config::{InputData, ModelConfig, SegmentId, TranscriptEntry};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct ContextWindowSegment {
    show_200k_marker: bool,
}

impl ContextWindowSegment {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the segment while Claude Code reports `exceeds_200k_tokens`: on a
    /// 1M window the percentage alone doesn't show that 200k was crossed.
    pub fn with_200k_marker(mut self, show: bool) -> Self {
        self.show_200k_marker = show;
        self
    }

    /// Get context limit for the specified model
    fn get_context_limit_for_model(model_id: &str) -> u32 {
        let model_config = ModelConfig::load();
        model_config.get_context_limit(model_id)
    }

    /// Resolve `(context limit, tokens currently in the context)`.
    ///
    /// Prefers what Claude Code reports on the payload: it knows the real
    /// window size (e.g. a 1M-token session), which can't be reliably
    /// inferred from the model id. Payloads without `context_window` (older
    /// Claude Code) keep the original behaviour: guess the window from the
    /// model id and read usage from the transcript.
    fn resolve_usage(input: &InputData) -> (u32, Option<u32>) {
        let Some(cw) = &input.context_window else {
            return (
                Self::get_context_limit_for_model(&input.model.id),
                parse_transcript_usage(&input.transcript_path),
            );
        };

        let limit = cw
            .context_window_size
            .filter(|&size| size > 0)
            .unwrap_or_else(|| Self::get_context_limit_for_model(&input.model.id));

        let used = match &cw.current_usage {
            // Usage of the most recent API call, counted exactly like the
            // transcript path: input + cache reads/writes + output.
            Some(usage) => Some(usage.clone().normalize().display_tokens()),
            // No API call yet in this session: Claude Code sends zeroed
            // totals (and null percentages), so there is nothing to show.
            None if cw.total_input_tokens == Some(0) => None,
            // No per-call breakdown: totals from older Claude Code builds may
            // be session-cumulative rather than the current context, and
            // without totals there is nothing to go on. Read the transcript
            // as before.
            None => parse_transcript_usage(&input.transcript_path),
        };

        (limit, used)
    }
}

impl Segment for ContextWindowSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let (context_limit, context_used_token_opt) = Self::resolve_usage(input);

        let (percentage_display, tokens_display) = match context_used_token_opt {
            Some(context_used_token) => {
                let context_used_rate = (context_used_token as f64 / context_limit as f64) * 100.0;

                let percentage = if context_used_rate.fract() == 0.0 {
                    format!("{:.0}%", context_used_rate)
                } else {
                    format!("{:.1}%", context_used_rate)
                };

                let tokens = if context_used_token >= 1000 {
                    let k_value = context_used_token as f64 / 1000.0;
                    if k_value.fract() == 0.0 {
                        format!("{}k", k_value as u32)
                    } else {
                        format!("{:.1}k", k_value)
                    }
                } else {
                    context_used_token.to_string()
                };

                (percentage, tokens)
            }
            None => {
                // No usage data available
                ("-".to_string(), "-".to_string())
            }
        };

        let mut metadata = HashMap::new();
        match context_used_token_opt {
            Some(context_used_token) => {
                let context_used_rate = (context_used_token as f64 / context_limit as f64) * 100.0;
                metadata.insert("tokens".to_string(), context_used_token.to_string());
                metadata.insert("percentage".to_string(), context_used_rate.to_string());
            }
            None => {
                metadata.insert("tokens".to_string(), "-".to_string());
                metadata.insert("percentage".to_string(), "-".to_string());
            }
        }
        metadata.insert("limit".to_string(), context_limit.to_string());
        metadata.insert("model".to_string(), input.model.id.clone());
        if let Some(exceeds) = input.exceeds_200k_tokens {
            metadata.insert("exceeds_200k_tokens".to_string(), exceeds.to_string());
        }

        // Claude Code's own flag rather than our count, which includes output
        // tokens; payloads without it (older Claude Code) never show the marker.
        let secondary = if self.show_200k_marker && input.exceeds_200k_tokens == Some(true) {
            "⚠ >200k".to_string()
        } else {
            String::new()
        };

        Some(SegmentData {
            primary: format!("{} · {} tokens", percentage_display, tokens_display),
            secondary,
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::ContextWindow
    }
}

fn parse_transcript_usage<P: AsRef<Path>>(transcript_path: P) -> Option<u32> {
    let path = transcript_path.as_ref();

    // Try to parse from current transcript file
    if let Some(usage) = try_parse_transcript_file(path) {
        return Some(usage);
    }

    // If file doesn't exist, try to find usage from project history
    if !path.exists() {
        if let Some(usage) = try_find_usage_from_project_history(path) {
            return Some(usage);
        }
    }

    None
}

fn try_parse_transcript_file(path: &Path) -> Option<u32> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default();

    if lines.is_empty() {
        return None;
    }

    // Check if the last line is a summary
    let last_line = lines.last()?.trim();
    if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(last_line) {
        if entry.r#type.as_deref() == Some("summary") {
            // Handle summary case: find usage by leafUuid
            if let Some(leaf_uuid) = &entry.leaf_uuid {
                let project_dir = path.parent()?;
                return find_usage_by_leaf_uuid(leaf_uuid, project_dir);
            }
        }
    }

    // Normal case: find the last assistant message in current file
    for line in lines.iter().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(line) {
            if entry.r#type.as_deref() == Some("assistant") {
                if let Some(message) = &entry.message {
                    if let Some(raw_usage) = &message.usage {
                        let normalized = raw_usage.clone().normalize();
                        return Some(normalized.display_tokens());
                    }
                }
            }
        }
    }

    None
}

fn find_usage_by_leaf_uuid(leaf_uuid: &str, project_dir: &Path) -> Option<u32> {
    // Search for the leafUuid across all session files in the project directory
    let entries = fs::read_dir(project_dir).ok()?;

    for entry in entries {
        let entry = entry.ok()?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }

        if let Some(usage) = search_uuid_in_file(&path, leaf_uuid) {
            return Some(usage);
        }
    }

    None
}

fn search_uuid_in_file(path: &Path, target_uuid: &str) -> Option<u32> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default();

    // Find the message with target_uuid
    for line in &lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(line) {
            if let Some(uuid) = &entry.uuid {
                if uuid == target_uuid {
                    // Found the target message, check its type
                    if entry.r#type.as_deref() == Some("assistant") {
                        // Direct assistant message with usage
                        if let Some(message) = &entry.message {
                            if let Some(raw_usage) = &message.usage {
                                let normalized = raw_usage.clone().normalize();
                                return Some(normalized.display_tokens());
                            }
                        }
                    } else if entry.r#type.as_deref() == Some("user") {
                        // User message, need to find the parent assistant message
                        if let Some(parent_uuid) = &entry.parent_uuid {
                            return find_assistant_message_by_uuid(&lines, parent_uuid);
                        }
                    }
                    break;
                }
            }
        }
    }

    None
}

fn find_assistant_message_by_uuid(lines: &[String], target_uuid: &str) -> Option<u32> {
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(line) {
            if let Some(uuid) = &entry.uuid {
                if uuid == target_uuid && entry.r#type.as_deref() == Some("assistant") {
                    if let Some(message) = &entry.message {
                        if let Some(raw_usage) = &message.usage {
                            let normalized = raw_usage.clone().normalize();
                            return Some(normalized.display_tokens());
                        }
                    }
                }
            }
        }
    }

    None
}

fn try_find_usage_from_project_history(transcript_path: &Path) -> Option<u32> {
    let project_dir = transcript_path.parent()?;

    // Find the most recent session file in the project directory
    let mut session_files: Vec<PathBuf> = Vec::new();
    let entries = fs::read_dir(project_dir).ok()?;

    for entry in entries {
        let entry = entry.ok()?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            session_files.push(path);
        }
    }

    if session_files.is_empty() {
        return None;
    }

    // Sort by modification time (most recent first)
    session_files.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH)
    });
    session_files.reverse();

    // Try to find usage from the most recent session
    for session_path in &session_files {
        if let Some(usage) = try_parse_transcript_file(session_path) {
            return Some(usage);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input_with(context_window: serde_json::Value) -> InputData {
        serde_json::from_value(json!({
            "model": {"id": "claude-sonnet-5", "display_name": "Sonnet 5"},
            "workspace": {"current_dir": "/tmp"},
            "transcript_path": "/nonexistent/transcript.jsonl",
            "context_window": context_window,
        }))
        .unwrap()
    }

    #[test]
    fn uses_payload_window_and_last_call_usage() {
        // Captured from Claude Code 2.1.275 in a 1M-context session, where the
        // model-id guess (200k) used to turn 7.5% into 37.6%.
        let input = input_with(json!({
            "total_input_tokens": 75086,
            "total_output_tokens": 144,
            "context_window_size": 1_000_000,
            "current_usage": {
                "input_tokens": 2,
                "output_tokens": 144,
                "cache_creation_input_tokens": 896,
                "cache_read_input_tokens": 74188
            },
            "used_percentage": 8,
            "remaining_percentage": 92
        }));

        assert_eq!(
            ContextWindowSegment::resolve_usage(&input),
            (1_000_000, Some(75_230))
        );
        let data = ContextWindowSegment::new().collect(&input).unwrap();
        assert_eq!(data.primary, "7.5% · 75.2k tokens");
    }

    #[test]
    fn marks_contexts_over_200k_only_when_enabled() {
        let over_200k = |exceeds: serde_json::Value| -> InputData {
            let mut input = input_with(json!({
                "total_input_tokens": 250_000,
                "total_output_tokens": 500,
                "context_window_size": 1_000_000,
                "current_usage": {
                    "input_tokens": 1_000,
                    "output_tokens": 500,
                    "cache_creation_input_tokens": 9_000,
                    "cache_read_input_tokens": 240_000
                }
            }));
            input.exceeds_200k_tokens = serde_json::from_value(exceeds).unwrap();
            input
        };
        let marked = ContextWindowSegment::new().with_200k_marker(true);

        assert_eq!(
            marked.collect(&over_200k(json!(true))).unwrap().secondary,
            "⚠ >200k"
        );
        assert_eq!(
            marked.collect(&over_200k(json!(false))).unwrap().secondary,
            ""
        );
        assert_eq!(
            marked.collect(&over_200k(json!(null))).unwrap().secondary,
            ""
        );
        assert_eq!(
            ContextWindowSegment::new()
                .collect(&over_200k(json!(true)))
                .unwrap()
                .secondary,
            ""
        );
    }

    #[test]
    fn falls_back_to_the_transcript_without_a_usage_breakdown() {
        let dir = std::env::temp_dir().join(format!("ccline-cw-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let transcript = dir.join("session.jsonl");
        fs::write(
            &transcript,
            r#"{"type":"assistant","message":{"usage":{"input_tokens":10,"cache_read_input_tokens":49990,"output_tokens":0}}}"#,
        )
        .unwrap();

        for context_window in [
            // older build: totals (possibly session-cumulative), no current_usage
            json!({"total_input_tokens": 900_000, "total_output_tokens": 5_000, "context_window_size": 1_000_000}),
            // no totals at all
            json!({"context_window_size": 1_000_000}),
        ] {
            let mut input = input_with(context_window);
            input.transcript_path = transcript.to_string_lossy().into_owned();
            assert_eq!(
                ContextWindowSegment::resolve_usage(&input),
                (1_000_000, Some(50_000))
            );
        }

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn shows_no_usage_before_first_response() {
        let input = input_with(json!({
            "total_input_tokens": 0,
            "total_output_tokens": 0,
            "context_window_size": 1_000_000,
            "current_usage": null,
            "used_percentage": null,
            "remaining_percentage": null
        }));

        assert_eq!(
            ContextWindowSegment::resolve_usage(&input),
            (1_000_000, None)
        );
        let data = ContextWindowSegment::new().collect(&input).unwrap();
        assert_eq!(data.primary, "- · - tokens");
    }
}
