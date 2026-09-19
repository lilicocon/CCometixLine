use crate::config::{
    AnsiColor, ColorConfig, IconConfig, SegmentConfig, SegmentId, TextStyleConfig,
};
use std::collections::HashMap;

pub fn model_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Model,
        enabled: true,
        icon: IconConfig {
            plain: "🤖".to_string(),
            nerd_font: "\u{e26d}".to_string(),
        },
        colors: ColorConfig {
            icon: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            text: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            background: Some(AnsiColor::Rgb {
                r: 136,
                g: 192,
                b: 208,
            }),
        },
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}

pub fn directory_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Directory,
        enabled: true,
        icon: IconConfig {
            plain: "📁".to_string(),
            nerd_font: "\u{f024b}".to_string(),
        },
        colors: ColorConfig {
            icon: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            text: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            background: Some(AnsiColor::Rgb {
                r: 163,
                g: 190,
                b: 140,
            }),
        },
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}

pub fn git_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Git,
        enabled: true,
        icon: IconConfig {
            plain: "🌿".to_string(),
            nerd_font: "\u{f02a2}".to_string(),
        },
        colors: ColorConfig {
            icon: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            text: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            background: Some(AnsiColor::Rgb {
                r: 129,
                g: 161,
                b: 193,
            }),
        },
        styles: TextStyleConfig::default(),
        options: {
            let mut opts = HashMap::new();
            opts.insert("show_sha".to_string(), serde_json::Value::Bool(false));
            opts
        },
    }
}

pub fn context_window_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::ContextWindow,
        enabled: true,
        icon: IconConfig {
            plain: "⚡️".to_string(),
            nerd_font: "\u{f49b}".to_string(),
        },
        colors: ColorConfig {
            icon: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            text: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            background: Some(AnsiColor::Rgb {
                r: 180,
                g: 142,
                b: 173,
            }),
        },
        styles: TextStyleConfig::default(),
        options: {
            let mut opts = HashMap::new();
            opts.insert(
                "show_200k_marker".to_string(),
                serde_json::Value::Bool(false),
            );
            opts
        },
    }
}

pub fn cost_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Cost,
        enabled: false,
        icon: IconConfig {
            plain: "💰".to_string(),
            nerd_font: "\u{eec1}".to_string(),
        },
        colors: ColorConfig {
            icon: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            text: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            background: Some(AnsiColor::Rgb {
                r: 235,
                g: 203,
                b: 139,
            }), // Nord yellow background
        },
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}

pub fn session_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Session,
        enabled: false,
        icon: IconConfig {
            plain: "⏱️".to_string(),
            nerd_font: "\u{f19bb}".to_string(),
        },
        colors: ColorConfig {
            icon: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            text: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            background: Some(AnsiColor::Rgb {
                r: 163,
                g: 190,
                b: 140,
            }), // Nord green background
        },
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}

pub fn output_style_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::OutputStyle,
        enabled: false,
        icon: IconConfig {
            plain: "🎯".to_string(),
            nerd_font: "\u{f12f5}".to_string(),
        },
        colors: ColorConfig {
            icon: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            text: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            background: Some(AnsiColor::Rgb {
                r: 136,
                g: 192,
                b: 208,
            }), // Nord cyan background
        },
        styles: TextStyleConfig::default(),
        options: HashMap::new(),
    }
}

pub fn usage_segment() -> SegmentConfig {
    SegmentConfig {
        id: SegmentId::Usage,
        enabled: false,
        icon: IconConfig {
            plain: "📊".to_string(),
            nerd_font: "\u{f0a9e}".to_string(),
        },
        colors: ColorConfig {
            icon: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            text: Some(AnsiColor::Rgb {
                r: 46,
                g: 52,
                b: 64,
            }),
            background: Some(AnsiColor::Rgb {
                r: 235,
                g: 203,
                b: 139,
            }), // Nord yellow background
        },
        styles: TextStyleConfig::default(),
        options: {
            let mut opts = HashMap::new();
            opts.insert(
                "api_base_url".to_string(),
                serde_json::Value::String("https://api.anthropic.com".to_string()),
            );
            opts.insert(
                "cache_duration".to_string(),
                serde_json::Value::Number(180.into()),
            );
            opts.insert("timeout".to_string(), serde_json::Value::Number(2.into()));
            opts.insert(
                "show_five_hour_reset".to_string(),
                serde_json::Value::Bool(false),
            );
            opts
        },
    }
}

pub fn prompt_cache_segment() -> SegmentConfig {
    // Cost's palette: both are about what the session spends
    let base = cost_segment();
    SegmentConfig {
        id: SegmentId::PromptCache,
        enabled: false,
        icon: IconConfig {
            plain: "♻️".to_string(),
            nerd_font: "\u{f00e8}".to_string(), // nf-md-cached
        },
        colors: base.colors,
        styles: base.styles,
        options: HashMap::new(),
    }
}

pub fn mode_segment() -> SegmentConfig {
    // Model's palette: effort and thinking are model settings
    let base = model_segment();
    SegmentConfig {
        id: SegmentId::Mode,
        enabled: false,
        icon: IconConfig {
            plain: "🧠".to_string(),
            nerd_font: "\u{f09d1}".to_string(), // nf-md-brain
        },
        colors: base.colors,
        styles: base.styles,
        options: HashMap::new(),
    }
}

pub fn usage_weekly_segment() -> SegmentConfig {
    // Context window's palette, so the 7-day window reads apart from the 5-hour usage next to it
    let base = context_window_segment();
    SegmentConfig {
        id: SegmentId::UsageWeekly,
        enabled: false,
        icon: IconConfig {
            plain: "📅".to_string(),
            nerd_font: "\u{f0a9e}".to_string(), // circle_slice_1; filled to the 7-day share at render
        },
        colors: base.colors,
        styles: base.styles,
        options: HashMap::new(),
    }
}
