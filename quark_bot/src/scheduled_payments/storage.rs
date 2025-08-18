use crate::scheduled_payments::dto::{PendingPaymentWizardState, ScheduledPaymentRecord};
use sled::{Db, IVec, Tree};

const SCHEDULED_PAYMENTS_TREE: &str = "scheduled_payments";
const SCHEDULED_PAYMENT_PENDING_TREE: &str = "scheduled_payment_pending";

#[derive(Clone)]
pub struct ScheduledPaymentsStorage {
    pub scheduled: Tree,
    pub pending: Tree,
}

impl ScheduledPaymentsStorage {
    pub fn new(db: &Db) -> sled::Result<Self> {
        let scheduled = db.open_tree(SCHEDULED_PAYMENTS_TREE)?;
        let pending = db.open_tree(SCHEDULED_PAYMENT_PENDING_TREE)?;
        Ok(Self { scheduled, pending })
    }

    pub fn put_schedule(&self, record: &ScheduledPaymentRecord) -> sled::Result<()> {
        let key = record.id.as_bytes();
        let bytes = bincode::encode_to_vec(record, bincode::config::standard()).unwrap();
        self.scheduled.insert(key, bytes)?;
        Ok(())
    }

    pub fn get_schedule(&self, id: &str) -> Option<ScheduledPaymentRecord> {
        self.scheduled
            .get(id.as_bytes())
            .ok()
            .flatten()
            .and_then(|ivec: IVec| {
                bincode::decode_from_slice::<ScheduledPaymentRecord, _>(
                    &ivec,
                    bincode::config::standard(),
                )
                .ok()
                .map(|(v, _)| v)
            })
    }

    pub fn list_schedules_for_group(&self, group_id: i64) -> Vec<ScheduledPaymentRecord> {
        let mut out = Vec::new();
        for kv in self.scheduled.iter() {
            if let Ok((_k, ivec)) = kv {
                if let Ok((rec, _)) = bincode::decode_from_slice::<ScheduledPaymentRecord, _>(
                    &ivec,
                    bincode::config::standard(),
                ) {
                    if rec.group_id == group_id && rec.active {
                        out.push(rec);
                    }
                }
            }
        }
        out
    }

    pub fn put_pending(&self, key: (&i64, &i64), state: &PendingPaymentWizardState) -> sled::Result<()> {
        let k = Self::pending_key_bytes(key);
        let bytes = bincode::encode_to_vec(state, bincode::config::standard()).unwrap();
        self.pending.insert(k, bytes)?;
        Ok(())
    }

    pub fn get_pending(&self, key: (&i64, &i64)) -> Option<PendingPaymentWizardState> {
        let k = Self::pending_key_bytes(key);
        self.pending.get(k).ok().flatten().and_then(|ivec: IVec| {
            bincode::decode_from_slice::<PendingPaymentWizardState, _>(&ivec, bincode::config::standard())
                .ok()
                .map(|(v, _)| v)
        })
    }

    pub fn delete_pending(&self, key: (&i64, &i64)) -> sled::Result<()> {
        let k = Self::pending_key_bytes(key);
        self.pending.remove(k)?;
        Ok(())
    }

    fn pending_key_bytes(key: (&i64, &i64)) -> Vec<u8> {
        let mut v = Vec::with_capacity(16);
        v.extend_from_slice(&key.0.to_be_bytes());
        v.extend_from_slice(&key.1.to_be_bytes());
        v
    }
}


