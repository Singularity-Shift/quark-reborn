use crate::{dependencies::BotDependencies, payment::dto::PaymentPrefs};
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

    pub async fn get_payment_token(
        &self,
        id: String,
        bot_deps: &BotDependencies,
    ) -> Option<PaymentPrefs> {
        let session = self.db.get(id).unwrap_or(None);
        if session.is_none() {
            return None;
        }
        let session = session.unwrap();
        let session = serde_json::from_slice::<PaymentPrefs>(&session);

        if session.is_err() {
            return None;
        }

        let session = session.unwrap();

        let token_list = bot_deps.panora.aptos.get_fees_currency_payment_list().await;
        if token_list.is_err() {
            return None;
        }
        let token_list = token_list.unwrap();
        let token_is_in_list = token_list
            .into_iter()
            .find(|t| session.currency.starts_with(t));

        if token_is_in_list.is_none() {
            return None;
        }

        Some(session)
    }

    pub fn set_payment_token(&self, id: String, value: PaymentPrefs) {
        self.db
            .insert(id, serde_json::to_vec(&value).unwrap())
            .unwrap();
    }
}
