use chrono::Utc;
use quark_core::helpers::dto::{CoinVersion, CreateDaoRequest};
use teloxide::{prelude::*, types::Message};
use uuid::Uuid;

use crate::{group::handler::Group, panora::handler::Panora, services::handler::Services};

pub async fn execute_create_dao(
    arguments: &serde_json::Value,
    bot: Bot,
    msg: Message,
    service: Services,
    group_id: Option<String>,
    group: Group,
    panora: Panora,
) -> String {
    if group_id.is_none() {
        return "❌ Group ID is required".to_string();
    }

    let group_id = group_id.unwrap();

    let group_id_parsed = ChatId(group_id.parse::<i64>().unwrap());

    let auth = group.get_credentials(&group_id_parsed);

    if auth.is_none() {
        return "❌ Error getting credentials, maybe the group is not logged in".to_string();
    }

    let auth = auth.unwrap();

    let user = msg.from.as_ref();

    if user.is_none() {
        return "❌ User is required".to_string();
    }

    let user = user.unwrap();

    let admin_ids = bot.get_chat_administrators(msg.chat.id).await;

    let admin_ids = match admin_ids {
        Ok(ids) => ids,
        Err(e) => {
            return format!("❌ Error getting chat administrators: {}", e);
        }
    };

    if admin_ids.is_empty() {
        return "❌ Error getting chat administrators".to_string();
    }

    let is_admin = admin_ids.iter().any(|admin| admin.user.id == user.id);

    if !is_admin {
        return "❌ You are not an admin of this group".to_string();
    }

    let name = arguments["name"].as_str();

    if name.is_none() {
        return "❌ Name is required".to_string();
    }

    let description = arguments["description"].as_str();

    if description.is_none() {
        return "❌ Description is required".to_string();
    }

    let options = arguments["options"].as_array();

    if options.is_none() {
        return "❌ Options are required".to_string();
    }

    let options = options.unwrap();

    if options.is_empty() {
        return "❌ Options are required".to_string();
    }

    let options = options
        .iter()
        .map(|option| option.as_str().unwrap().to_string())
        .collect::<Vec<String>>();

    let start_date = arguments["start_date"].as_str();

    if start_date.is_none() {
        return "❌ Start date is required".to_string();
    }

    let end_date = arguments["end_date"].as_str();

    if end_date.is_none() {
        return "❌ End date is required".to_string();
    }

    let symbol = arguments["symbol"].as_str();

    if symbol.is_none() {
        return "❌ Symbol is required".to_string();
    }

    let symbol = symbol.unwrap();

    if start_date.is_none() {
        return "❌ Start date is required".to_string();
    }

    let start_date = start_date.unwrap();

    let start_date = start_date.parse::<u64>();

    if start_date.is_err() {
        return "❌ Start date is invalid".to_string();
    }

    let start_date = start_date.unwrap();

    if end_date.is_none() {
        return "❌ End date is invalid".to_string();
    }

    let end_date = end_date.unwrap().parse::<u64>();

    if end_date.is_err() {
        return "❌ End date is invalid".to_string();
    }

    let end_date = end_date.unwrap();

    if start_date > end_date {
        return "❌ Start date must be before end date".to_string();
    }

    let now = Utc::now().timestamp();

    if start_date < now as u64 {
        return "❌ Start date must be in the future".to_string();
    }

    let token = panora.get_token_by_symbol(symbol).await;

    if token.is_err() {
        return "❌ Error getting token address".to_string();
    }

    let token = token.unwrap();

    let version = if token.token_address.is_some() {
        CoinVersion::V1
    } else {
        CoinVersion::V2
    };

    let dao_id = Uuid::new_v4().to_string();

    let request = CreateDaoRequest {
        name: name.unwrap().to_string(),
        description: description.unwrap().to_string(),
        options,
        start_date,
        end_date,
        group_id,
        dao_id,
        version,
        currency: if token.token_address.is_some() {
            token.token_address.unwrap()
        } else {
            token.fa_address
        },
    };

    let response = service.create_dao(auth.jwt, request).await;

    if response.is_err() {
        return "❌ Error creating DAO".to_string();
    }

    return format!("DAO created successfully: {}", response.unwrap().hash);
}
