use quark_core::helpers::dto::{CoinVersion, PayUsersRequest};
use serde::{Deserialize, Serialize};
use sled::{Db, IVec};

const TREE_NAME: &str = "pending_transactions";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PendingTransaction {
    pub transaction_id: String,         // Unique UUID for this transaction
    pub amount: u64,                    // Amount in smallest units
    pub users: Vec<String>,             // User addresses  
    pub coin_type: String,              // Token address/type
    pub version: CoinVersion,           // V1 or V2
    pub jwt_token: String,              // JWT for authentication
    pub is_group_transfer: bool,        // Whether this is a group or individual transfer
    pub symbol: String,                 // Token symbol for display
    pub user_addresses: Vec<String>,    // Recipient addresses
    pub original_usernames: Vec<String>, // Original usernames for display
    pub per_user_amount: f64,           // Amount per user (for display)
    pub created_at: u64,                // Timestamp when created
    pub expires_at: u64,                // Timestamp when transaction expires
    pub chat_id: i64,                   // Telegram chat ID where the message was sent
    pub message_id: i32,                // Telegram message ID of the transaction message
}

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

    /// Get all pending transactions (for cleanup)
    pub fn get_all_pending_transactions(&self) -> Vec<(String, PendingTransaction)> {
        self.tree
            .iter()
            .filter_map(|result| {
                if let Ok((key, value)) = result {
                    let key_str = String::from_utf8(key.to_vec()).ok()?;
                    let transaction: PendingTransaction = serde_json::from_slice(&value).ok()?;
                    Some((key_str, transaction))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Remove expired transactions and return them for message updating
    pub fn cleanup_expired_transactions(&self) -> sled::Result<(usize, Vec<PendingTransaction>)> {
        let all_transactions = self.get_all_pending_transactions();
        let mut removed_count = 0;
        let mut expired_transactions = Vec::new();

        for (key, transaction) in all_transactions {
            if Self::is_expired(&transaction) {
                // Only collect transactions that have valid message IDs (not placeholder 0)
                if transaction.message_id != 0 {
                    expired_transactions.push(transaction.clone());
                }
                self.tree.remove(key.as_bytes())?;
                removed_count += 1;
            }
        }

        Ok((removed_count, expired_transactions))
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
}