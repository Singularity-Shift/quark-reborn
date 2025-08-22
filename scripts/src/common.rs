// Common constants and types shared between download and upload modules

// Define target files with flexible matching
pub const TARGET_FILES: &[(&str, &str, &[&str])] = &[
    (
        "../quark_bot/src/ai/prompt.rs",
        "prompt.rs",
        &["prompt", "prompt.rs", "ai_prompt", "ai_prompt.rs"],
    ),
    (
        "../quark_bot/src/ai/moderation/moderation_service.rs",
        "moderation_service.rs",
        &[
            "moderation_service",
            "moderation_service.rs",
            "moderation",
            "moderation.rs",
        ],
    ),
    (
        "../quark_bot/src/ai/moderation/overrides.rs",
        "overrides.rs",
        &[
            "overrides",
            "overrides.rs",
            "moderation_overrides",
            "moderation_overrides.rs",
        ],
    ),
    (
        "../quark_bot/src/ai/schedule_guard/schedule_guard_service.rs",
        "schedule_guard_service.rs",
        &[
            "schedule_guard_service",
            "schedule_guard_service.rs",
            "schedule_guard",
            "schedule_guard.rs",
        ],
    ),
];

pub type TargetFile = (&'static str, &'static str, &'static [&'static str]);
