use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "Open the Aptos Connect app.")]
    AptosConnect,
    #[command(description = "Log in as a user (DM only).", parse_with = "split")]
    LoginUser,
    #[command(description = "Group login (under development).", parse_with = "split")]
    LoginGroup,
    #[command(description = "Display this text.")]
    Help,
    #[command(description = "Upload files to your vector store (DM only).")]
    AddFiles,
    #[command(description = "List files in your vector store (DM only).")]
    ListFiles,
    #[command(description = "Start a new conversation thread.")]
    NewChat,
    #[command(description = "Send a prompt to the bot.")]
    C(String),
    #[command(description = "Send a prompt to the O3 reasoning model.")]
    R(String),
    #[command(description = "Show example prompts.")]
    PromptExamples,
}

#[derive(Debug, Clone, Default)]
pub enum QuarkState {
    #[default]
    Chat,
}
