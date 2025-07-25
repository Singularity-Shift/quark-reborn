Below is a **step-by-step implementation checklist** that plugs straight into the current *quark-reborn* code layout and uses **`teloxide::dispatching::dialogue::InMemStorage`** (RAM-only) for the rolling 20-message history.

---

## 1 · Create a dedicated module

**`quark_bot/src/message_history/mod.rs`**

```rust
use serde::{Deserialize, Serialize};
use teloxide::{
    dispatching::dialogue::{InMemStorage, Storage},
    types::{ChatId, Message},
};

/// One stored line.
#[derive(Clone, Serialize, Deserialize)]
pub struct MessageEntry {
    pub sender: Option<String>,
    pub text:   String,
}

/// Per-chat buffer (max 20).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct MessageHistory(pub Vec<MessageEntry>);

impl MessageHistory {
    pub fn push(&mut self, entry: MessageEntry) {
        if self.0.len() >= 20 {
            self.0.remove(0);           // drop oldest
        }
        self.0.push(entry);
    }
}

/// Handy alias used everywhere else.
pub type HistoryStorage = InMemStorage<MessageHistory>;

/// Log a new group text.
pub async fn log(
    msg: &Message,
    storage: &HistoryStorage,
) -> anyhow::Result<()> {
    if msg.chat.is_private() || msg.text().is_none() {
        return Ok(());                  // skip DMs & non-text
    }

    let sender = msg.from()
        .and_then(|u| u.username.clone().or_else(|| Some(u.first_name.clone())));

    // Fetch, mutate, save.
    let mut state = storage
        .read(msg.chat.id)
        .await?
        .unwrap_or_default();
    state.push(MessageEntry {
        sender,
        text: msg.text().unwrap().to_owned(),
    });
    storage.update(msg.chat.id, state).await?;
    Ok(())
}

/// Fetch the buffer (may be empty).
pub async fn fetch(
    chat_id: ChatId,
    storage: &HistoryStorage,
) -> Vec<MessageEntry> {
    storage.read(chat_id).await.unwrap_or_default().0
}
```

---

## 2 · Wire the storage into the DI container

**`quark_bot/src/main.rs`**

```rust
use crate::message_history::HistoryStorage;   // new

Dispatcher::builder(bot.clone(), handler_tree())
    .dependencies(dptree::deps![
        InMemStorage::<QuarkState>::new(),
        HistoryStorage::new(),               // ← add here
        /* the rest stay unchanged */
    ])
```

The `.dependencies(...)` injection pattern is exactly what Teloxide’s docs show for in-memory storages. ([Docs.rs][1])

---

## 3 · Log every incoming message

Modify the **signature** of `bot/handler.rs::handle_message`:

```rust
pub async fn handle_message(
    bot: Bot,
    msg: Message,
    ai: AI,
    media_aggregator: Arc<MediaGroupAggregator>,
    cmd_collector: Arc<CommandImageCollector>,
    db: Db,
    auth: Auth,
    group: Group,
    panora: Panora,
    services: Services,
    history: HistoryStorage,                    // ← new
) -> AnyResult<()> {
```

Add the call right at the top (line 1103 now):

```rust
    // ── 0. Remember this line for context ───────────────
    crate::message_history::log(&msg, &history).await?;
```

Everything else in `handle_message` stays intact.

Because `HistoryStorage::new()` is in the dependency list, Teloxide will automatically inject it into any endpoint that asks for it.

---

## 4 · Expose a tool to the LLM

### 4-a · Tool definition

**`ai/tools.rs`**

```rust
/// Get recent group messages – returns last ≈20 lines
pub fn get_recent_messages_tool() -> Tool {
    Tool::function(
        "get_recent_messages",
        "Retrieve the most recent messages (up to 20) from THIS Telegram group chat.",
        json!({}),          // no args
    )
}
```

Append it to `get_all_custom_tools()`.

### 4-b · Tool execution

Add to **`ai/actions.rs`**

```rust
use crate::message_history::HistoryStorage;

pub async fn execute_get_recent_messages(
    msg: Message,
    history: HistoryStorage,
) -> String {
    if msg.chat.is_private() {
        return "This tool is only available in group chats.".into();
    }
    let lines = crate::message_history::fetch(msg.chat.id, &history).await;
    if lines.is_empty() {
        return "(No recent messages stored.)".into();
    }
    lines
        .into_iter()
        .map(|e| match e.sender {
            Some(name) => format!("{name}: {}", e.text),
            None       => e.text,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

### 4-c · Bridge into the dispatcher

Expand the **`execute_custom_tool`** signature in `ai/tools.rs` to accept `history: HistoryStorage`, then add the route:

```rust
"get_recent_messages" =>
    execute_get_recent_messages(msg, history).await,
```

Finally, **update the call site** in `ai/handler.rs` (the only place that calls `execute_custom_tool`) by passing `history.clone()`.

---

## 5 · Token-safety (optional)

If you want a belt-and-suspenders guard, clip long messages **before** pushing in `message_history::log`:

```rust
let mut text = msg.text().unwrap().to_owned();
const MAX_CHARS: usize = 200;
if text.chars().count() > MAX_CHARS {
    text.truncate(MAX_CHARS);
    text.push('…');
}
```

---

## 6 · Compilation touch-ups

1. `Cargo.toml` already has `serde` and `teloxide`, so no new deps.
2. Add `mod message_history;` to `quark_bot/src/lib.rs` (or the relevant `mod` tree).
3. Run `cargo check`—`dptree` will satisfy all injections automatically.

---

## 7 · Verification matrix

| Scenario          | Expected behaviour                                           |
| ----------------- | ------------------------------------------------------------ |
| Brand-new group   | Tool returns “(No recent messages…)”.                        |
| ≤ 20 normal texts | Tool echoes exactly those lines in order.                    |
| > 20 texts        | Oldest ones roll off.                                        |
| DM chat           | Tool refuses (“only in group chats”).                        |
| Bot restart       | History is **cleared** (RAM only) — document this in README. |

---

### Outcome

This patch set keeps histories **isolated per group**, uses **pure RAM** via Teloxide’s `InMemStorage`, touches only a handful of files, and leaves the rest of the bot logic undisturbed.

[1]: https://docs.rs/teloxide/latest/teloxide/dispatching/index.html "teloxide::dispatching - Rust"
