pub fn simulate_aptos_contract_call(user_id: i64) -> String {
    format!(
        "Aptos contract called for user {}. Transaction: SUCCESS ✅ (simulated)",
        user_id
    )
} 