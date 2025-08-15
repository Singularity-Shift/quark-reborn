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
                "Group Overrides (these supersede defaults; strict precedence applies: Group Disallowed > Group Allowed > Default Rules):\n\n",
                "Group Disallowed (always flag if any match):\n",
                "{disallowed_list}\n\n",
                "Group Allowed (do not flag due to default rules if matched and no Group Disallowed matched):\n",
                "{allowed_list}\n\n",
                "Decision rule:\n",
                "- If any Group Disallowed item matches the message (including semantically), return 'F'.\n",
                "- Else if any Group Allowed item matches the message (including semantically), return 'P'.\n",
                "- Else apply Default Rules below.\n"
            ),
            disallowed_list = disallowed_list,
            allowed_list = allowed_list,
        )
    } else {
        String::new()
    }
}


