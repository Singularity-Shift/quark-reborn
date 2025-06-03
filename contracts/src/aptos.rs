pub fn simulate_aptos_contract_call(user_id: i64) -> String {
    let msg = format!(
        "Aptos contract called for user {}. Transaction: SUCCESS ✅ (simulated)",
        user_id
    );
    println!("Simulated Aptos contract call: {}", msg);
    msg
} 