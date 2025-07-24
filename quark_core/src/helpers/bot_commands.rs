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
    #[command(description = "Login as a group admin.", parse_with = "split")]
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
    #[command(description = "Send a prompt to the bot in a group.")]
    G(String),
    #[command(description = "Send a prompt to the your selected reasoning model.")]
    R(String),
    #[command(description = "Show example prompts.")]
    PromptExamples,
    #[command(description = "Select reasoning model (O-series) and effort level.")]
    SelectReasoningModel,
    #[command(description = "Select chat model (4-series) and temperature.")]
    SelectModel,
    #[command(description = "View your current model preferences (DM only).")]
    MySettings,
    // Change Monitor to Sentinel
    #[command(description = "Monitor system status (on/off).", rename = "sentinel")]
    Sentinel(String),
    #[command(description = "Moderate content (reply to message).")]
    Mod,
    #[command(description = "Display the moderation rules to avoid getting muted.")]
    ModerationRules,
    #[command(description = "Get your wallet address.")]
    WalletAddress,
    #[command(description = "Get your balance of a token.")]
    Balance(String),
    #[command(description = "Get the group's wallet address.")]
    GroupWalletAddress,
    #[command(description = "Get the group's balance of a token.")]
    GroupBalance(String),
    #[command(description = "Display model pricing information.")]
    Prices,
}

#[derive(Debug, Clone, Default)]
pub enum QuarkState {
    #[default]
    Chat,
}
