use quark_core::helpers::dto::{CoinVersion, PayUsersRequest};
use serde::{Deserialize, Serialize};
use sled::{Db, IVec};

const TREE_NAME: &str = "pending_transactions";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PendingTransaction {
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
        Ok(())
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