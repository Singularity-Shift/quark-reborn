use std::time::Duration;
use quark_core::helpers::dto::PayUsersRequest;
use sled::{Db, IVec};
use teloxide::{Bot, prelude::*};
use tokio::time::timeout;

use super::dto::PendingTransaction;

const TREE_NAME: &str = "pending_transactions";

#[derive(Clone)]
pub struct PendingTransactions {
    tree: sled::Tree,
}

impl PendingTransactions {
    pub fn new(db: &Db) -> sled::Result<Self> {
        let tree = db.open_tree(TREE_NAME)?;
        Ok(Self { tree })
    }

    /// Create a composite key from user_id and group_id
    fn create_key(user_id: i64, group_id: Option<i64>) -> String {
        match group_id {
            Some(gid) => format!("{}:{}", user_id, gid),
            None => format!("{}:0", user_id),
        }
    }

    pub fn set_pending_transaction(
        &self,
        user_id: i64,
        group_id: Option<i64>,
        transaction: &PendingTransaction,
    ) -> sled::Result<()> {
        let key = Self::create_key(user_id, group_id);
        let encoded = serde_json::to_vec(transaction).unwrap();
        self.tree.insert(key.as_bytes(), encoded)?;
        
        // Verify the write completed by attempting to read it back
        // Retry up to 10 times with small delays to handle eventual consistency
        for attempt in 0..10 {
            if self.get_pending_transaction(user_id, group_id).is_some() {
                return Ok(());
            }
            
            if attempt < 9 {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        
        // If we get here, the write verification failed
        Err(sled::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to verify transaction storage after write"
        )))
    }

    pub fn get_pending_transaction(
        &self,
        user_id: i64,
        group_id: Option<i64>,
    ) -> Option<PendingTransaction> {
        let key = Self::create_key(user_id, group_id);
        self.tree
            .get(key.as_bytes())
            .ok()
            .flatten()
            .and_then(|ivec: IVec| {
                serde_json::from_slice(&ivec).ok()
            })
    }

    pub fn delete_pending_transaction(
        &self,
        user_id: i64,
        group_id: Option<i64>,
    ) -> sled::Result<()> {
        let key = Self::create_key(user_id, group_id);
        self.tree.remove(key.as_bytes())?;
        Ok(())
    }

    /// Check if a transaction has expired
    pub fn is_expired(transaction: &PendingTransaction) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > transaction.expires_at
    }



    /// Update message_id for an existing pending transaction
    pub fn update_transaction_message_id(
        &self,
        user_id: i64,
        group_id: Option<i64>,
        message_id: i32,
    ) -> sled::Result<()> {
        let key = Self::create_key(user_id, group_id);
        
        if let Some(mut transaction) = self.get_pending_transaction(user_id, group_id) {
            transaction.message_id = message_id;
            let encoded = serde_json::to_vec(&transaction).unwrap();
            self.tree.insert(key.as_bytes(), encoded)?;
            
            // Verify the write completed by attempting to read it back
            // Retry up to 10 times with small delays to handle eventual consistency
            for attempt in 0..10 {
                if let Some(updated_transaction) = self.get_pending_transaction(user_id, group_id) {
                    if updated_transaction.message_id == message_id {
                        return Ok(());
                    }
                }
                
                if attempt < 9 {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
            
            // If we get here, the update verification failed
            Err(sled::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to verify transaction message ID update"
            )))
        } else {
            Err(sled::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Transaction not found for message ID update"
            )))
        }
    }

    /// Convert PendingTransaction to PayUsersRequest for service calls
    pub fn to_pay_users_request(transaction: &PendingTransaction) -> PayUsersRequest {
        PayUsersRequest {
            amount: transaction.amount,
            users: transaction.user_addresses.clone(),
            coin_type: transaction.coin_type.clone(),
            version: transaction.version.clone(),
        }
    }

    /// Start timeout for transaction - automatically cleans up if user doesn't respond
    pub async fn start_transaction_timeout(
        &self,
        bot: Bot,
        user_id: i64,
        group_id: Option<i64>,
        transaction: &PendingTransaction,
    ) {
        let transaction_clone = transaction.clone();
        
        // Use tokio::time::timeout to wait for 60 seconds
        // We create a future that never completes (representing waiting for user action)
        let wait_forever = std::future::pending::<()>();
        
        // Wrap it with a 60-second timeout - this is the direct timeout call
        if let Err(_) = timeout(Duration::from_secs(60), wait_forever).await {
            // Timeout occurred - user didn't act within 60 seconds
            log::info!("Transaction {} timed out after 60 seconds", transaction_clone.transaction_id);
            
            // Check if transaction still exists and clean it up
            if let Some(expired_transaction) = self.get_pending_transaction(user_id, group_id) {
                // Verify this is still the same transaction
                if expired_transaction.transaction_id == transaction_clone.transaction_id {
                    // Remove the expired transaction
                    if let Err(e) = self.delete_pending_transaction(user_id, group_id) {
                        log::error!("Failed to delete expired transaction {}: {}", transaction_clone.transaction_id, e);
                        return;
                    }
                    
                    log::info!("Automatically expired and removed transaction: {}", transaction_clone.transaction_id);
                    
                    // Update the message to show expiration (only if message_id is valid)
                    if expired_transaction.message_id != 0 {
                        let recipients_text = if expired_transaction.original_usernames.len() == 1 {
                            format!("@{}", expired_transaction.original_usernames[0])
                        } else {
                            expired_transaction.original_usernames.iter()
                                .map(|username| format!("@{}", username))
                                .collect::<Vec<_>>()
                                .join(", ")
                        };

                        let expired_message = format!(
                            "⏰ <b>Transaction expired</b>\n\n💰 {:.2} {} to {} was not sent.\n\n<i>Transactions expire after 1 minute for security.</i>",
                            expired_transaction.per_user_amount * expired_transaction.original_usernames.len() as f64,
                            expired_transaction.symbol,
                            recipients_text
                        );
                        
                        if let Err(e) = bot.edit_message_text(
                            teloxide::types::ChatId(expired_transaction.chat_id),
                            teloxide::types::MessageId(expired_transaction.message_id),
                            expired_message
                        )
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await 
                        {
                            log::warn!("Failed to edit expired transaction message for chat {} message {}: {}", 
                                expired_transaction.chat_id, expired_transaction.message_id, e);
                        } else {
                            log::info!("Successfully updated expired transaction message for chat {} message {}", 
                                expired_transaction.chat_id, expired_transaction.message_id);
                        }
                    }
                } else {
                    log::debug!("Transaction ID mismatch during timeout cleanup - transaction was likely replaced");
                }
            } else {
                log::debug!("Transaction {} was already removed before timeout", transaction_clone.transaction_id);
            }
        }
    }
}