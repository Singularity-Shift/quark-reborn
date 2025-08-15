use crate::payment::dto::PaymentPrefs;
use sled::{Db, Tree};

#[derive(Clone)]
pub struct Payment {
    db: Tree,
}

impl Payment {
    pub fn new(db: &Db) -> sled::Result<Self> {
        let tree = db.open_tree("payment")?;
        Ok(Self { db: tree })
    }

    pub fn get_payment_token(&self, id: String) -> Option<PaymentPrefs> {
        let session = self.db.get(id).unwrap_or(None);
        if session.is_none() {
            return None;
        }
        let session = session.unwrap();
        let session = serde_json::from_slice::<PaymentPrefs>(&session).unwrap();
        Some(session)
    }

    pub fn set_payment_token(&self, id: String, value: PaymentPrefs) {
        self.db
            .insert(id, serde_json::to_vec(&value).unwrap())
            .unwrap();
    }
}
