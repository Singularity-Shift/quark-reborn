use anyhow::Result;
use open_ai_rust_responses_by_sshift::{Client, Model, Request, ReasoningEffort, Verbosity};
use teloxide::{Bot, prelude::*, types::Message};

use crate::ai::moderation::dto::{ModerationOverrides, ModerationResult};
use crate::ai::moderation::overrides::build_override_section;

#[derive(Clone)]
pub struct ModerationService {
    pub(crate) client: Client,
}

impl ModerationService {
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::new(&api_key)?;
        Ok(Self { client })
    }

    pub async fn moderate_message(
        &self,
        message_text: &str,
        bot: &Bot,
        original_msg: &Message,
        replied_msg: &Message,
        overrides: Option<ModerationOverrides>,
    ) -> Result<ModerationResult> {
        // Check if the user who sent the replied message has admin role
        if let Some(user) = &replied_msg.from {
            let user_id = user.id;

            // Get chat administrators
            if let Ok(admins) = bot.get_chat_administrators(original_msg.chat.id).await {
                let is_admin = admins.iter().any(|member| member.user.id == user_id);

                if is_admin {
                    // Admin users automatically pass moderation (no API call, no tokens used)
                    return Ok(ModerationResult {
                        verdict: "P".to_string(),
                        total_tokens: 0,
                    });
                }
            }
        }

        // Build group override section if provided
        let override_section = build_override_section(overrides);

        // Proceed with AI moderation for non-admin users
        let prompt = format!(
            r#"# CRYPTO COMMUNITY CONTENT MODERATOR

## TASK
You are a high-precision content moderator for a cryptocurrency community. Analyze the message and return EXACTLY one character:
- 'F' if the message violates ANY rule
- 'P' if the message is acceptable

**CRITICAL:** Return only 'F' or 'P' - no explanations, reasoning, or additional text.

## DECISION PROCESS
Apply rules in strict priority order:
1. Group Override Rules (if provided)
2. Default Moderation Rules (below)

Think step-by-step but return only the final decision.

{override_section}

## DEFAULT MODERATION RULES
Flag as 'F' if the message contains ANY of the following:

### 1. PROMOTIONAL/COMMERCIAL CONTENT
**Violations:**
- Offering services, products, or paid access
- Promising exclusive opportunities or deals
- Positioning as authority to gain trust ("I'm an expert", "Trust me")
- Any commercial solicitation or sales pitch

**Examples:**
- "Selling WL spots — DM for price"
- "Join my premium alpha/signals group — 0.1 ETH/week"
- "VIP Discord access for early airdrops — limited slots"
- "I'll shill your token to my followers — paid promo"
- "CMC/CG listing boost service — guaranteed exposure"
- "Paid mentorship/trading course — enroll now"
- "Whitelist pass for sale; flipping guaranteed"
- "Unlock private channels for $50/month"

### 2. PRIVATE COMMUNICATION REQUESTS
**Violations:**
- Requesting to move conversation to private channels
- Sharing personal contact information unsolicited
- Asking others to contact privately

**Examples:**
- "DM me / message me privately / contact me directly"
- "Let's take this to TG/WhatsApp/Signal/WeChat DMs"
- "Telegram: @user | WhatsApp: +1 555‑123…"
- "Scan this QR to chat with support"
- Obfuscated: "D|\\/| me", "d​m me" (zero‑width), "DМ me" (Cyrillic M), "msg m3 pr1v8ly"

### 3. PHISHING/SCAM INDICATORS
**Violations:**
- External links + call-to-action words (vote, sign, claim, mint, verify, connect, airdrop, whitelist)
- Urgency language with or without links ("urgent", "act now", "limited time", "hurry")
- Requests for wallet connections, private keys, or seed phrases
- Impersonation of official accounts or support

**Examples:**
- "Verify your account: hxxps://binаnce.vip-login.support" (homoglyph + deceptive domain)
- "Connect wallet to claim airdrop: https[:]//wormhole-verify[.]io"
- "Approve unlimited USDT to continue" / "Sign permit (EIP‑2612) to receive tokens"
- "Urgent: restore access in 30 minutes or lose funds"
- "Official support here — share your seed/private key to unlock"
- "You won! Claim prize: hxxps://secure-crypto-site[.]com"
- "Scan this QR to restore your wallet/app access"
- "MetaMask security check — login and confirm" (fake ad / cloned site)

### 4. FINANCIAL SCAMS
**Violations:**
- Guaranteed return promises
- Investment advice with unrealistic claims
- Rug pull indicators (unverified projects with urgent investment calls)

**Examples:**
- "Send 1 BTC, get 2 BTC back (limited time)"
- "Guaranteed 10% daily returns — risk‑free"
- "Insider pump at 9pm — buy now before it moons"
- "Arbitrage bot: 100% success rate — deposit to start"
- "New token launch — invest now, 1000% APY"
- "Double your ETH event — official giveaway"

### 5. DEFI/NFT DRAINERS & WALLET APPROVAL TRAPS
**Violations:**
- Requests for broad wallet approvals on unknown contracts
- Unaudited staking or minting requests
- Suspicious permit/signature requests

**Examples:**
- "Click 'SetApprovalForAll' to mint faster"
- "Approve unlimited spending to qualify for drop"
- "Sign EIP‑712 'Permit' to receive airdrop rewards"
- "Stake to unlock rewards" on unknown/unaudited contracts
- "Free mint" links that request broad approvals

### 6. ADDRESS & APP IMPERSONATION
**Violations:**
- Address poisoning attempts
- Fake app downloads
- Impersonation of official services

**Examples:**
- Address poisoning: "Use my new addr: 0xABCD…90EF" (looks similar to frequent address)
- Clipboard malware telltale: pasted address silently changes to lookalike
- Fake apps: "Download MetaMask Pro here" (off‑store APK/IPA, cloned branding)
- Fake Google/X ads: sponsored result shows official brand but redirects to drainer

## SEMANTIC INTERPRETATION GUIDELINES
- Consider synonyms, paraphrases, and equivalent expressions
- Account for typos, abbreviations (e.g., "u" for "you", "ur" for "your")
- Recognize obfuscation attempts (sp@cing, special characters)
- Focus on intent and context, not just exact wording
- Consider crypto-specific terminology and slang

**Obfuscation Pattern Examples:**
- **Leetspeak:** "Fr33 b1tc0in g1v3away", "s3ll1ng dr*gs"
- **Homoglyphs:** "Clаim yоur рrizе" (Cyrillic a/o/p), "cоntact me" (Cyrillic o)
- **Zero-width characters:** "w​a​l​l​e​t", "v​e​r​i​f​y" (invisible spaces)
- **Spacing:** "c l a i m  a i r d r o p", "F R E E B i t c o i n s"
- **Hxxps/brackets:** "hxxps://site[.]com", "https[:]//domain[.]io"
- **Emojis/code words:** "🌽 alpha," "flip your bread," "wl spot," "dr0p"
- **Character substitution:** "D|\\/| me", "msg m3 pr1v8ly", "acc0unt"

## ACCEPTABLE CONTENT
**Do NOT flag these as violations:**
- General crypto discussions and education
- Legitimate project announcements from verified sources
- Technical questions and answers
- Market analysis and opinions (without investment advice)
- Community engagement ("gm", "lfg", "wagmi", etc.)
- Sharing public information or news articles
- Asking genuine questions about crypto concepts

**Examples of Acceptable Content:**
- "Guide: how to spot airdrop scams" (educational)
- "News: exchange warns of fake ads" (legitimate news links)
- "What is EIP‑712?" (technical Q&A without CTAs)
- "How to secure your wallet?" (educational discussion)
- "Market analysis: BTC showing bullish divergence" (analysis without investment advice)
- "New DeFi protocol launches on Ethereum" (project announcement)
- "Anyone tried the new DEX aggregator?" (community discussion)

## EDGE CASE HANDLING
- **Ambiguous cases:** If uncertain, consider context and err on the side of community safety
- **Educational content:** Allow educational discussions even if they mention risky topics
- **News sharing:** Allow sharing of legitimate news articles even if they contain links
- **Official announcements:** Consider source credibility and verification status

**Specific Edge Case Examples:**
- **Educational vs. Promotion:** "Learn about DeFi risks" (OK) vs "Learn from my paid course" (flag)
- **News vs. Scam:** "Article about recent rug pulls" (OK) vs "Join this project before it rugs" (flag)
- **Technical Discussion vs. Investment Advice:** "How does staking work?" (OK) vs "Stake here for guaranteed returns" (flag)
- **Community vs. Private:** "Anyone want to discuss trading strategies?" (OK) vs "DM me for trading signals" (flag)

Message to analyze: "{}""#,
            message_text, override_section = override_section
        );

        let request = Request::builder()
            .model(Model::GPT5Nano)
            .input(prompt)
            .verbosity(Verbosity::Low)
            .reasoning_effort(ReasoningEffort::Minimal)
            .max_output_tokens(500)
            .build();

        let response = self.client.responses.create(request).await?;
        let result = response.output_text().trim().to_uppercase();

        // Extract token usage
        let total_tokens = if let Some(usage) = &response.usage {
            usage.total_tokens
        } else {
            0
        };

        // Ensure we only return P or F
        let verdict = if result.contains('F') {
            "F".to_string()
        } else {
            "P".to_string()
        };

        Ok(ModerationResult {
            verdict,
            total_tokens,
        })
    }
}


