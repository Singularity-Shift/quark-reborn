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
                "## GROUP OVERRIDE RULES\n",
                "**Priority: Group Disallowed > Group Allowed > Default Rules**\n\n",
                "### Group Disallowed Items (ALWAYS flag as 'F' if ANY match):\n",
                "{disallowed_list}\n\n",
                "### Group Allowed Items (NEVER flag if matched, unless Group Disallowed also matches):\n",
                "{allowed_list}\n\n",
                "### Semantic Matching Guidelines:\n",
                "- Consider synonyms, paraphrases, and contextually equivalent expressions\n",
                "- Examples: \"DM me\" = \"message me privately\" = \"contact me directly\"\n",
                "- Account for typos, abbreviations, and alternative spellings\n",
                "- Consider intent behind the message, not just exact wording\n\n",
                "### Normalization & Obfuscation Handling:\n",
                "- Case-insensitive; ignore extra spaces and punctuation where it doesn't change meaning\n",
                "- Normalize URLs: treat 'hxxps' as 'https', '[.]' as '.', ignore 'www' when comparing domains\n",
                "- Collapse zero-width/invisible characters; treat homoglyphs (e.g., Cyrillic o) as equivalent\n",
                "- Recognize leetspeak and simple character substitutions (e.g., 'acc0unt', 'D|\\/| me')\n\n",
                "Examples: \"Fr33 b1tc0in g1v3away\", \"Clаim yоur рrizе\", \"hxxps://site[.]com\", \"d\u{200B}m me\", \"c l a i m  a i r d r o p\"\n\n",
                "### Decision Process:\n",
                "1. Check if message semantically matches ANY Group Disallowed item → Return 'F'\n",
                "2. If no disallowed match, check if message semantically matches ANY Group Allowed item → Return 'P'\n",
                "3. If no override matches, proceed to Default Rules below\n\n"
            ),
            disallowed_list = disallowed_list,
            allowed_list = allowed_list,
        )
    } else {
        String::new()
    }
}


