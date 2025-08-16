use crate::ai::moderation::dto::ModerationOverrides;

pub fn build_override_section(overrides: Option<ModerationOverrides>) -> String {
    if let Some(o) = overrides {
        let allowed_list = if o.allowed_items.is_empty() {
            "- (none)".to_string()
        } else {
            o.allowed_items
                .iter()
                .map(|x| format!("- {}", x))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let disallowed_list = if o.disallowed_items.is_empty() {
            "- (none)".to_string()
        } else {
            o.disallowed_items
                .iter()
                .map(|x| format!("- {}", x))
                .collect::<Vec<_>>()
                .join("\n")
        };
        format!(
            concat!(
                "[INSERT YOUR OVERRIDE RULES PROMPTING HERE]",
                "\n\n",
                "Disallowed: {disallowed_list}\n",
                "Allowed: {allowed_list}\n"
            ),
            disallowed_list = disallowed_list,
            allowed_list = allowed_list,
        )
    } else {
        String::new()
    }
}
