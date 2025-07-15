use regex::Regex;

pub fn extract_url_from_markdown(text: &str) -> Option<String> {
    // First try to extract from markdown format [text](url)
    let re_markdown = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
    if let Some(captures) = re_markdown.captures(text) {
        return Some(captures.get(2)?.as_str().to_string());
    }

    // If no markdown URL found, try to extract plain URLs
    let re_plain_url = Regex::new(r"https?://[^\s]+").unwrap();
    if let Some(mat) = re_plain_url.find(text) {
        return Some(mat.as_str().to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_url_from_markdown() {
        // Test markdown format
        let markdown_text = "Check this [link](https://example.com) for more info";
        assert_eq!(
            extract_url_from_markdown(markdown_text),
            Some("https://example.com".to_string())
        );

        // Test plain URL
        let plain_url_text = "Your resource account has been funded with 0.1 APT from your main wallet. You can check the details here: https://sshiftgpt.tunn.dev/fund?coin=APT&amount=0.1\n\nIf you need further assistance, just let me know.";
        assert_eq!(
            extract_url_from_markdown(plain_url_text),
            Some("https://sshiftgpt.tunn.dev/fund?coin=APT&amount=0.1".to_string())
        );

        // Test no URL
        let no_url_text = "This text has no URL";
        assert_eq!(extract_url_from_markdown(no_url_text), None);
    }
}
