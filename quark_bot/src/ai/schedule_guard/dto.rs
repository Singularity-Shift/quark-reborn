#[derive(Debug, Clone)]
pub struct ScheduleGuardResult {
	pub verdict: String, // "P" or "F"
	pub reason: Option<String>,
	pub total_tokens: u32,
}


