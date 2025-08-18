use std::sync::Arc;

use crate::message_history::handler::HistoryStorage;
use crate::payment::dto::PaymentPrefs;
use crate::scheduled_prompts::storage::ScheduledStorage;
use crate::scheduled_payments::storage::ScheduledPaymentsStorage;
use crate::{
    ai::handler::AI, assets::media_aggregator::MediaGroupAggregator, credentials::handler::Auth,
    dao::dao::Dao, group::handler::Group, panora::handler::Panora, payment::payment::Payment,
    pending_transactions::handler::PendingTransactions, services::handler::Services,
    user_conversation::handler::UserConversations, yield_ai::yield_ai::YieldAI,
};
use tokio_cron_scheduler::JobScheduler;
use crate::ai::schedule_guard::schedule_guard_service::ScheduleGuardService;
use crate::ai::moderation::ModerationService;

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
    pub scheduled_storage: ScheduledStorage,
    pub scheduled_payments: ScheduledPaymentsStorage,
    pub media_aggregator: Arc<MediaGroupAggregator>,
    pub history_storage: HistoryStorage,
    pub pending_transactions: PendingTransactions,
    pub yield_ai: YieldAI,
    pub scheduler: Arc<JobScheduler>,
    pub payment: Payment,
    pub default_payment_prefs: PaymentPrefs,
    pub schedule_guard: ScheduleGuardService,
    pub moderation: ModerationService,
}
