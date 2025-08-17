use anyhow::Result;
use open_ai_rust_responses_by_sshift::{Client, Model, Request, ReasoningEffort, Verbosity};

use super::dto::ScheduleGuardResult;

#[derive(Clone)]
pub struct ScheduleGuardService {
	client: Client,
}

impl ScheduleGuardService {
	pub fn new(api_key: String) -> Result<Self> {
		let client = Client::new(&api_key)?;
		Ok(Self { client })
	}

	pub async fn check_prompt(&self, prompt_text: &str) -> Result<ScheduleGuardResult> {
		let guard_prompt = format!(
			r#"[INSERT YOUR SCHEDULE GUARD PROMPTING HERE]"#,
			msg = prompt_text
		);

		let request = Request::builder()
			.model(Model::GPT5Nano)
			.input(guard_prompt)
			.verbosity(Verbosity::Low)
			.reasoning_effort(ReasoningEffort::Minimal)
			.max_output_tokens(500)
			.build();

		let response = self.client.responses.create(request).await?;
		let raw = response.output_text().trim().to_string();

		let total_tokens = response.usage.as_ref().map(|u| u.total_tokens).unwrap_or(0);

		let upper = raw.to_uppercase();
		if upper == "P" {
			return Ok(ScheduleGuardResult {
				verdict: "P".to_string(),
				reason: None,
				total_tokens,
			});
		}

		// Expect formats like: "F: reason" or just "F"
		let mut reason: Option<String> = None;
		if raw.starts_with('F') {
			let rest = raw.chars().skip(1).collect::<String>();
			let trimmed = rest.trim_start_matches([':', '-', ' ']).trim();
			if !trimmed.is_empty() {
				reason = Some(trimmed.to_string());
			}
			return Ok(ScheduleGuardResult {
				verdict: "F".to_string(),
				reason,
				total_tokens,
			});
		}

		// Default to pass if output is unexpected
		Ok(ScheduleGuardResult {
			verdict: "P".to_string(),
			reason: None,
			total_tokens,
		})
	}
}
