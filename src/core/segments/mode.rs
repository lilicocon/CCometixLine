use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use std::collections::HashMap;

/// Session settings that change what each turn costs and how long it takes:
/// reasoning effort, extended thinking and fast mode, e.g. `high · think`.
#[derive(Default)]
pub struct ModeSegment;

impl ModeSegment {
    pub fn new() -> Self {
        Self
    }
}

impl Segment for ModeSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let effort = input
            .effort
            .as_ref()
            .and_then(|effort| effort.level.as_deref())
            .filter(|level| !level.is_empty());
        let thinking = input
            .thinking
            .as_ref()
            .and_then(|thinking| thinking.enabled)
            == Some(true);
        let fast = input.fast_mode == Some(true);

        let mut parts = Vec::new();
        if let Some(level) = effort {
            parts.push(level);
        }
        if thinking {
            parts.push("think");
        }
        if fast {
            parts.push("fast");
        }
        // Nothing enabled, or an older Claude Code that sends none of these
        if parts.is_empty() {
            return None;
        }

        let mut metadata = HashMap::new();
        if let Some(level) = effort {
            metadata.insert("effort".to_string(), level.to_string());
        }
        metadata.insert("thinking".to_string(), thinking.to_string());
        metadata.insert("fast_mode".to_string(), fast.to_string());

        Some(SegmentData {
            primary: parts.join(" · "),
            secondary: String::new(),
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input_with(fields: serde_json::Value) -> InputData {
        let mut payload = json!({
            "model": {"id": "claude-opus-5", "display_name": "Opus 5"},
            "workspace": {"current_dir": "/tmp"},
            "transcript_path": "/nonexistent/transcript.jsonl",
        });
        payload
            .as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        serde_json::from_value(payload).unwrap()
    }

    #[test]
    fn shows_effort_and_enabled_flags() {
        let captured = input_with(json!({
            "effort": {"level": "high"},
            "thinking": {"enabled": true},
            "fast_mode": false
        }));
        let everything = input_with(json!({
            "effort": {"level": "max"},
            "thinking": {"enabled": true},
            "fast_mode": true
        }));

        assert_eq!(
            ModeSegment::new().collect(&captured).unwrap().primary,
            "high · think"
        );
        assert_eq!(
            ModeSegment::new().collect(&everything).unwrap().primary,
            "max · think · fast"
        );
    }

    #[test]
    fn model_without_effort_support() {
        let input = input_with(json!({"thinking": {"enabled": true}, "fast_mode": true}));

        assert_eq!(
            ModeSegment::new().collect(&input).unwrap().primary,
            "think · fast"
        );
    }

    #[test]
    fn hidden_when_nothing_is_on_or_reported() {
        let all_off = input_with(json!({"thinking": {"enabled": false}, "fast_mode": false}));

        assert!(ModeSegment::new().collect(&all_off).is_none());
        assert!(ModeSegment::new().collect(&input_with(json!({}))).is_none());
    }
}
