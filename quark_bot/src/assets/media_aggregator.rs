use dashmap::DashMap;
use sled::Db;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;

pub struct MediaGroupAggregator {
    // Key: media_group_id
    // Value: (Vec of messages in the group, debounce task handle)
    groups: DashMap<String, (Vec<Message>, tokio::task::JoinHandle<()>)>,
}

impl MediaGroupAggregator {
    pub fn new(_bot: Bot, _db: Db) -> Self {
        Self {
            groups: DashMap::new(),
        }
    }

    pub async fn add_message(self: Arc<Self>, msg: Message) {
        let media_group_id = if let Some(id) = msg.media_group_id() {
            id.to_string()
        } else {
            return;
        };

        let mut entry = self
            .groups
            .entry(media_group_id.clone())
            .or_insert_with(|| (Vec::new(), tokio::spawn(async {})));

        // A new message has arrived for the group, so cancel the previous debounce task.
        entry.value().1.abort();

        // Add the new message to the group.
        entry.value_mut().0.push(msg);

        // Clone the Arc to move it into the new task.
        let aggregator_clone = self.clone();

        // Start a new debounce task.
        let handle = tokio::spawn(async move {
            // Wait for a short period to see if more messages arrive for this group.
            tokio::time::sleep(Duration::from_millis(2000)).await;

            // The timer has elapsed, so we can now process the group.
            if let Some((_, (messages, _))) = aggregator_clone.groups.remove(&media_group_id) {
                log::error!(
                    "Error handling grouped chat for {}, {:?}",
                    media_group_id,
                    messages
                );
            }
        });

        // Store the new task's handle.
        entry.value_mut().1 = handle;
    }
}
