use regex::Regex;

pub fn extract_url_from_markdown(text: &str) -> Option<String> {
    let re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
    if let Some(captures) = re.captures(text) {
        Some(captures.get(2)?.as_str().to_string())
    } else {
        None
    }
}
