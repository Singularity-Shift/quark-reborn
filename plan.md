🎸 *A tight riff on one theme: “log-in, qualify, store.”*
Below is a **laser-focused blueprint** for X (Twitter) onboarding inside **quark-reborn-main**—everything the scraper will later need to confirm who liked/RT’d/replied.

---

## 1  Flow at a glance

```
/loginx  (DM) ──► OAuth2 PKCE ──► /callback
     │                            │
state = "<telegram_id>|<nonce>"   │
     ▼                            ▼
  save verifier + state      fetch profile, qualify
                             ▼
                       store TwitterUser in sled
```

---

## 2  Implementation details

### 2.1  Bot side — `quark_bot`

| File              | Key additions                                                                                                                              |                                                                                                                                         |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| `src/commands.rs` | `/loginx` command:<br>• Reject if not `ChatKind::Private`.<br>• `let (verifier, challenge) = pkce::pair();`<br>• \`let state = format!("{} | {}", chat\_id, nonce());`<br>• Persist `{state, verifier}`in **sled tree`oauth\_states\`\*\* with 15-min TTL.<br>• Reply with auth URL. |

**Auth URL builder** (`quark_core::twitter::auth::build_auth_url`)

```rust
format!(
  "https://twitter.com/i/oauth2/authorize\
   ?response_type=code\
   &client_id={CID}\
   &redirect_uri={URI}\
   &scope=tweet.read%20users.read\
   &state={state}\
   &code_challenge={challenge}\
   &code_challenge_method=S256"
)
```

Docs reference ([docs.x.com][1])

---

### 2.2  Server side — `quark_server`

| File                    | Key additions                                                                                                                                                                                                                                                                                                                                                                                                             |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/routes.rs`         | `POST /callback` handler (Axum).                                                                                                                                                                                                                                                                                                                                                                                          |
| `src/handlers/oauth.rs` | 1. Parse `state`, split into `telegram_id, nonce`.<br>2. Fetch `verifier` from `oauth_states`; abort if missing.<br>3. `POST https://api.twitter.com/2/oauth2/token` with `code`, `verifier` → `AccessToken`.<br>4. `GET https://api.twitter.com/2/users/me` → profile data ([docs.x.com][2]).<br>5. `GET /1.1/users/profile_banner.json?...` for banner.<br>6. Compute `qualifies` (rules below).<br>7. Persist to sled. |

---

## 3  Eligibility logic (unchanged)

```rust
qualifies = followers >= 50
         && has_profile_pic
         && has_banner_pic
         && !verified;
```

---

## 4  Data model (sled tree `twitter_auth_v2`)

```rust
#[derive(Serialize, Deserialize)]
pub struct TwitterUserV2 {
    pub telegram: String,      // "username" sans @
    pub twitter_handle: String,
    pub twitter_id: u64,       // numeric UID – lets scraper dedup
    pub access_token: Encrypted<String>,
    pub refresh_token: Option<Encrypted<String>>,
    pub scopes: Vec<String>,   // tweet.read, users.read
    pub follower_count: u32,
    pub has_profile_pic: bool,
    pub has_banner_pic: bool,
    pub verified: bool,
    pub qualifies: bool,
    pub checked_at: u64,       // epoch secs
    pub version: u8,           // = 2
}
```

> **Why these fields?**
> • `twitter_handle` → matches strings scraped from UI
> • `twitter_id` → fallback if X UI drops handles
> • `qualifies` → single flag used by cron when awarding raids

---

## 5  Security notes

* **Encrypt** tokens with `age` or `ring` before saving (key from `TW_TOKEN_KEY` env var).
* Store `oauth_states` as a separate sled tree and purge expired entries on startup.
* Never log raw tokens.

---

## 6  Hooks for the scraper (`quark_server::scraper`)

When `collect_users(tweet_id)` returns `HashSet<String>` of `@handles`:

```rust
for handle in handles {
    if let Some(user) = twitter_auth_tree.get(handle)? {
        if user.qualifies { raid.participants.insert(handle); }
    }
}
```

If X ever switches to numeric IDs in UI, extend scraper to emit `twitter_id` too and match both keys.

---

## 7  Testing checklist (focused)

* [ ] `/loginx` DM returns a valid PKCE URL (state saved).
* [ ] Visiting URL → X consent → `/callback` stores `TwitterUserV2`.
* [ ] Re-run `/loginx`; existing record updates follower count.
* [ ] Unqualified user gets `qualifies = false`.
* [ ] Scraper matches a qualifying handle from stored data.

---

🎶 *Steel strings quiet—mission trimmed to the chord you asked for: log-in, qualify, store.* Let me know when you want the next verse on the scraper itself or token refresh mechanics.

[1]: https://docs.x.com/resources/fundamentals/authentication/oauth-2-0/authorization-code?utm_source=chatgpt.com "OAuth 2.0 Authorization Code Flow with PKCE - X Docs"
[2]: https://docs.x.com/x-api/users/user-lookup-me?utm_source=chatgpt.com "User lookup me - X - X Docs"
