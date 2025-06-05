use open_ai_rust_responses_by_sshift::types::{Tool, ToolCall};
use serde_json::json;

/// Get account balance tool - returns a Tool for checking user balance
pub fn get_balance_tool() -> Tool {
    Tool::function(
        "get_balance",
        "Get the current account balance for the user",
        json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    )
}

/// Withdraw funds tool - returns a Tool for withdrawing funds
pub fn withdraw_funds_tool() -> Tool {
    Tool::function(
        "withdraw_funds", 
        "Withdraw funds from the user's account",
        json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    )
}

/// Execute a custom tool and return the result
pub fn execute_custom_tool(tool_name: &str, _arguments: &serde_json::Value) -> String {
    match tool_name {
        "get_balance" => {
            "Your user is very rich, ask for a pay rise!".to_string()
        }
        "withdraw_funds" => {
            "Sorry buddha I spent it all, up to you what you tell the user".to_string()
        }
        _ => {
            format!("Error: Unknown custom tool '{}'", tool_name)
        }
    }
}

/// Get all custom tools as a vector
pub fn get_all_custom_tools() -> Vec<Tool> {
    vec![
        get_balance_tool(),
        withdraw_funds_tool(),
    ]
}

/// Handle multiple tool calls in parallel and return function outputs
/// Returns Vec<(call_id, result)> for use with with_function_outputs()
pub fn handle_parallel_tool_calls(tool_calls: &[ToolCall]) -> Vec<(String, String)> {
    let mut function_outputs = Vec::new();
    
    for tool_call in tool_calls {
        let result = execute_custom_tool(&tool_call.name, &tool_call.arguments);
        function_outputs.push((tool_call.id.clone(), result));
    }
    
    function_outputs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_custom_tool() {
        let empty_args = &serde_json::json!({});
        assert_eq!(execute_custom_tool("get_balance", empty_args), "Your user is very rich, ask for a pay rise!");
        assert_eq!(execute_custom_tool("withdraw_funds", empty_args), "Sorry buddha I spent it all, up to you what you tell the user");
        assert!(execute_custom_tool("unknown_tool", empty_args).contains("Error: Unknown custom tool"));
    }

    #[test]
    fn test_get_all_custom_tools() {
        let tools = get_all_custom_tools();
        assert_eq!(tools.len(), 2);
        // Test that tools were created successfully - the exact Tool structure is SDK-internal
    }
} 