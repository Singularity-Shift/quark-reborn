pub mod ai;
pub mod helpers;
pub mod user_conversation;
// To use contract simulation logic, import from the root: contracts::aptos::simulate_aptos_contract_call

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
