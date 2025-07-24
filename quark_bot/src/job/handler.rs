use std::env;

use chrono::Utc;
use teloxide::{Bot, prelude::*, types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode}};
use tokio_cron_scheduler::Job;
use aptos_rust_sdk_types::api_types::view::ViewRequest;

use crate::{
    dao::dao::Dao,
    panora::handler::Panora,
};
use quark_core::helpers::dto::CoinVersion;

// Helper function to escape special characters for MarkdownV2
fn escape_markdown_v2(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|' | '{' | '}' | '.' | '!' => format!("\\{}", c),
            _ => c.to_string(),
        })
        .collect()
}

pub fn job_token_list(panora: Panora) -> Job {
    Job::new_async("0 0 * * * *", move |_uuid, _l| {
        let panora = panora.clone();
        Box::pin(async move {
            match panora.set_panora_token_list().await {
                Ok(_) => log::info!("Successfully updated Panora token list"),
                Err(e) => log::error!("Failed to update Panora token list: {}", e),
            }
        })
    })
    .expect("Failed to create cron job")
}

pub fn job_token_ai_fees(panora: Panora) -> Job {
    Job::new_async("0 * * * * *", move |_uuid, _l| {
        let panora = panora.clone();
        Box::pin(async move {
            let token_address = panora.aptos.get_token_address().await.unwrap();
            match panora.set_token_ai_fees(&token_address).await {
                Ok(_) => log::info!("Successfully updated Panora token AI fees"),
                Err(e) => log::error!("Failed to update Panora token AI fees: {}", e),
            }
        })
    })
    .expect("Failed to create cron job")
}

pub fn job_active_daos(dao: Dao, bot: Bot) -> Job {
    let base_url = env::var("APP_URL").unwrap();
    Job::new_async("0 */10 * * * *", move |_uuid, _l| {
        let base_url = base_url.clone();
        let dao = dao.clone();
        let bot = bot.clone();
        Box::pin(async move {
            let daos = dao.get_active_daos().unwrap();

            for dao_entry in daos {
                let group_id = dao_entry.group_id.clone();

                let chat_group_id = ChatId(group_id.parse::<i64>().unwrap());
                let admin_preferences = dao.get_dao_admin_preferences(group_id.clone()).unwrap();
                let now = Utc::now().timestamp() as u64;

                if dao_entry.last_active_notification
                    + admin_preferences.interval_active_dao_notifications
                    < now
                {
                    // Create inline keyboard with voting options
                    let mut keyboard_buttons = Vec::new();
                    
                    for (index, option) in dao_entry.options.iter().enumerate() {
                        let base_url = format!(
                            "{}/dao?group_id={}&dao_id={}&choice_id={}&coin_type={}&coin_version={}",
                            base_url,
                            group_id,
                            dao_entry.dao_id,
                            index,
                            dao_entry.coin_type,
                            match dao_entry.version {
                                CoinVersion::V1 => "V1",
                                CoinVersion::V2 => "V2",
                            }
                        );
                        
                        // Create a row with both mini app and browser buttons for each option
                        let option_row = vec![
                            // Mini App button
                            InlineKeyboardButton::web_app(
                                format!("📱 {}", option),
                                teloxide::types::WebAppInfo { url: base_url.parse().unwrap() }
                            ),
                            // External browser button
                            InlineKeyboardButton::url(
                                format!("🌐 {}", option),
                                base_url.parse().unwrap()
                            ),
                        ];
                        
                        keyboard_buttons.push(option_row);
                    }

                    // Add a separator row with voting instructions
                    keyboard_buttons.push(vec![
                        InlineKeyboardButton::callback(
                            "ℹ️ How to Vote",
                            "voting_help"
                        )
                    ]);

                    let keyboard = InlineKeyboardMarkup::new(keyboard_buttons);

                    // Create rich message text
                    let message_text = format!(
                        "🏛️ {}\n\n📝 {}\n\n⏰ Voting ends at timestamp: {}\n\n👆 Choose your preferred way to vote:\n📱 Mini App (opens in Telegram)\n🌐 Browser (opens externally)",
                        dao_entry.name,
                        dao_entry.description,
                        dao_entry.end_date
                    );

                    bot.send_message(chat_group_id, message_text)
                        .reply_markup(keyboard)
                        .await
                        .unwrap();

                    dao.update_last_active_notification(dao_entry.dao_id).unwrap();
                }
            }
        })
    })
    .expect("Failed to create cron job")
}

pub fn job_daos_results(panora: Panora, bot: Bot, dao: Dao) -> Job {
    Job::new_async("0 0 * * * *", move |_uuid, _l| {
        let panora = panora.clone();
        let bot = bot.clone();
        let dao = dao.clone();
        Box::pin(async move {
            log::info!("DAO results job executed at {}", Utc::now());
            
            // Get finished DAOs that haven't been notified yet
            let daos = match dao.get_dao_results() {
                Ok(daos) => daos,
                Err(e) => {
                    log::error!("Failed to get active DAOs: {}", e);
                    return;
                }
            };

            for dao_entry in daos {
                // Check if DAO has ended and results haven't been sent
                log::info!("Processing finished DAO: {}", dao_entry.dao_id);
                
                match fetch_and_send_dao_results(&panora, &bot, &dao, &dao_entry).await {
                    Ok(_) => {
                        log::info!("Successfully sent DAO results for: {}", dao_entry.dao_id);
                        if let Err(e) = dao.update_result_notified(dao_entry.dao_id.clone()) {
                            log::error!("Failed to update result_notified for DAO {}: {}", dao_entry.dao_id, e);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to send DAO results for {}: {}", dao_entry.dao_id, e);
                    }
                }
            }
        })
    })
    .expect("Failed to create cron job")
}

async fn fetch_and_send_dao_results(
    panora: &Panora,
    bot: &Bot,
    _dao: &Dao,
    dao_entry: &crate::dao::dto::DaoEntry,
) -> anyhow::Result<()> {
    let group_id = dao_entry.group_id.clone();
    let chat_group_id = ChatId(group_id.parse::<i64>()?);
    
    // Prepare view request based on DAO version
    let view_function = match dao_entry.version {
        CoinVersion::V1 => "get_group_dao_v1",
        CoinVersion::V2 => "get_group_dao_v2",
    };
    
    let view_request = ViewRequest {
        function: format!("{}::group::{}", panora.aptos.contract_address, view_function),
        type_arguments: vec![],
        arguments: vec![
            serde_json::to_value(&group_id)?,
            serde_json::to_value(&dao_entry.dao_id)?,
        ],
    };
    
    // Call the smart contract
    let response = panora.aptos.node.view_function(view_request).await?;
    let dao_data = response.into_inner();
    
    // Parse the response (assuming it returns the DAO data)
    if let Some(dao_data_array) = dao_data.as_array() {
        if let Some(dao_info) = dao_data_array.first() {
            let dao_info: serde_json::Value = dao_info.clone();
        
            // Extract data from the response
            let empty_vec = vec![];
            let choices = dao_info["choices"].as_array().unwrap_or(&empty_vec);
            let choices_weights = dao_info["choices_weights"].as_array().unwrap_or(&empty_vec);
        
        // Find the winning option
        let mut max_votes = 0u64;
        let mut winning_index = 0;
        let mut total_votes = 0u64;
        
        for (index, weight) in choices_weights.iter().enumerate() {
            let votes = weight.as_u64().unwrap_or(0);
            total_votes += votes;
            if votes > max_votes {
                max_votes = votes;
                winning_index = index;
            }
        }
        
            // Create results message (using MarkdownV2 formatting)
            let mut results_text = format!(
                "🏆 *DAO VOTING RESULTS*\n\n🏛️ *{}*\n📝 {}\n\n📊 *Results:*\n",
                escape_markdown_v2(&dao_entry.name),
                escape_markdown_v2(&dao_entry.description)
            );
            
            for (index, choice) in choices.iter().enumerate() {
                let choice_name = choice.as_str().unwrap_or("Unknown");
                let votes = choices_weights[index].as_u64().unwrap_or(0);
                let percentage = if total_votes > 0 {
                    (votes as f64 / total_votes as f64 * 100.0).round() as u64
                } else {
                    0
                };
                
                let emoji = if index == winning_index { "🥇" } else { "📊" };
                results_text.push_str(&format!(
                    "{} *{}*: {} votes \\({}%\\)\n",
                    emoji, escape_markdown_v2(choice_name), votes, percentage
                ));
            }
            
            if total_votes > 0 {
                let winning_choice = choices[winning_index].as_str().unwrap_or("Unknown");
                results_text.push_str(&format!(
                    "\n🎉 *Winner: {}* with {} votes\\!\n📈 Total votes cast: {}",
                    escape_markdown_v2(winning_choice), max_votes, total_votes
                ));
            } else {
                results_text.push_str("\n❌ No votes were cast for this DAO\\.");
            }
        
            // Send the results message
            bot.send_message(chat_group_id, results_text)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
                
            log::info!("Sent DAO results for {} to group {}", dao_entry.dao_id, group_id);
        } else {
            log::warn!("No DAO data found in response for DAO: {}", dao_entry.dao_id);
            // Send a simple completion message
            bot.send_message(chat_group_id, format!("🏛️ DAO \"{}\" has ended.", dao_entry.name))
                .await?;
        }
    } else {
        log::warn!("No data returned from smart contract for DAO: {}", dao_entry.dao_id);
        // Send a simple completion message
        bot.send_message(chat_group_id, format!("🏛️ DAO \"{}\" has ended.", dao_entry.name))
            .await?;
    }
    
    Ok(())
}
