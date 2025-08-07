use std::sync::Arc;

use crate::message_history::handler::HistoryStorage;
use crate::{
    ai::handler::AI, assets::media_aggregator::MediaGroupAggregator, credentials::handler::Auth,
    dao::dao::Dao, group::handler::Group, panora::handler::Panora, 
    pending_transactions::handler::PendingTransactions, services::handler::Services,
    user_conversation::handler::UserConversations,
};

#[derive(Clone)]
pub struct BotDependencies {
    pub db: sled::Db,
    pub auth: Auth,
    pub service: Services,
    pub user_convos: UserConversations,
    pub user_model_prefs: crate::user_model_preferences::handler::UserModelPreferences,
    pub ai: AI,
    pub cmd_collector:
        std::sync::Arc<crate::assets::command_image_collector::CommandImageCollector>,
    pub panora: Panora,
    pub group: Group,
    pub dao: Dao,
    pub media_aggregator: Arc<MediaGroupAggregator>,
    pub history_storage: HistoryStorage,
    pub pending_transactions: PendingTransactions,
}
