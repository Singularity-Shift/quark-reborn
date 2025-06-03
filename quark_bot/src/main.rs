use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use quark_backend::ai::generate_response;
use contracts::aptos::simulate_aptos_contract_call;
use std::env;
use teloxide::net::Download;

#[derive(BotCommands, Clone)]
#[command(description = "These commands are supported:")]
enum Command {
    #[command(description = "Log in as a user (DM only).", parse_with = "split")]
    LoginUser,
    #[command(description = "Group login (under development).", parse_with = "split")]
    LoginGroup,
    #[command(description = "Display this text.")]
    Help,
    #[command(description = "Upload files to your vector store (DM only).")]
    AddFiles,
    #[command(description = "List files in your vector store (DM only).")]
    ListFiles,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    log::info!("Starting quark_bot...");

    let bot = Bot::from_env();
    let me = bot.get_me().await.expect("Failed to get bot info");
    let bot_username = me.user.username.expect("Bot has no username");
    let db = sled::open("quark_db").expect("Failed to open sled DB");
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

    let handler = Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
        let bot_username = bot_username.clone();
        let db = db.clone();
        let openai_api_key = openai_api_key.clone();
        async move {
            if let Some(text) = msg.text() {
                // /login_user command, only in private chat
                if (text == "/login_user" || text == format!("/login_user@{}", bot_username))
                    && msg.chat.is_private()
                {
                    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
                    let mention = if let Some(user) = msg.from.as_ref() {
                        format!("<a href=\"tg://user?id={}\">{}</a>", user.id.0, user.first_name)
                    } else {
                        format!("<a href=\"tg://user?id={}\">{}</a>", user_id, user_id)
                    };
                    let fake_address = format!("fake_address_{}", user_id);
                    let _ = simulate_aptos_contract_call(user_id); // log only
                    let reply = format!(
                        "logged in as {}, please add 📒 token to address <code>{}</code> to use the bot",
                        mention, fake_address
                    );
                    bot.send_message(msg.chat.id, reply)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                } else if let Some(stripped) = text.strip_prefix(&format!("/chat@{} ", bot_username))
                    .or_else(|| text.strip_prefix("/chat "))
                {
                    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
                    let reply = match generate_response(user_id, stripped, &db, &openai_api_key).await {
                        Ok(resp) => resp,
                        Err(e) => format!("[AI error]: {}", e),
                    };
                    bot.send_message(msg.chat.id, reply).await?;
                } else if text == "/chat" || text == format!("/chat@{}", bot_username) {
                    bot.send_message(msg.chat.id, "Usage: /chat <your message>").await?;
                }
                // /add_files command, only in private chat
                if (text == "/add_files" || text == format!("/add_files@{}", bot_username)) && msg.chat.is_private() {
                    bot.send_message(msg.chat.id, "📎 Please attach the files you wish to upload in your next message.\n\n✅ Supported: Documents, Photos, Videos, Audio files\n💡 You can send multiple files in one message!").await?;
                }
                if text == "/help" || text == format!("/help@{}", bot_username) {
                    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
                }
                // /login_group command
                if text == "/login_group" || text == format!("/login_group@{}", bot_username) {
                    bot.send_message(msg.chat.id, "under development").await?;
                }
                // /list_files command
                if text.starts_with("/list_files") && msg.chat.is_private() {
                    use quark_backend::ai::list_user_files_with_names;
                    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
                    let db = db.clone();
                    
                    // Check if user has a vector store
                    let user_convos = quark_backend::db::UserConversations::new(&db).unwrap();
                    if let Some(_vector_store_id) = user_convos.get_vector_store_id(user_id) {
                        match list_user_files_with_names(user_id, &db) {
                            Ok(files) => {
                                if files.is_empty() {
                                    bot.send_message(msg.chat.id, "Your vector store is empty. Use /add_files to upload documents.").await?;
                                } else {
                                    let file_list = files.iter()
                                        .enumerate()
                                        .map(|(i, file)| format!("{}. {} ({})", i + 1, file.name, file.id))
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    let response = format!("📁 Files in your vector store ({} total):\n\n{}\n\n💡 Files are tracked locally for reliable listing.", files.len(), file_list);
                                    bot.send_message(msg.chat.id, response).await?;
                                }
                            },
                            Err(e) => {
                                bot.send_message(msg.chat.id, format!("❌ Error listing files: {}", e)).await?;
                            }
                        }
                    } else {
                        bot.send_message(msg.chat.id, "You don't have a vector store yet. Use /add_files to upload your first documents.").await?;
                    }
                }
            }
            // Handle document upload in DM for /add_files
            if msg.chat.is_private() && (msg.document().is_some() || msg.photo().is_some() || msg.video().is_some() || msg.audio().is_some()) {
                use quark_backend::ai::upload_files_to_vector_store;
                use tokio::fs::File;
                let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
                let mut file_paths = Vec::new();
                
                // Handle document
                if let Some(document) = msg.document() {
                    let file_id = &document.file.id;
                    let file_info = bot.get_file(file_id).await?;
                    let filename = document.file_name.clone().unwrap_or_else(|| "document.bin".to_string());
                    let file_path = format!("/tmp/{}_{}", user_id, filename);
                    let mut file = File::create(&file_path).await.map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
                    bot.download_file(&file_info.path, &mut file).await.map_err(|e| teloxide::RequestError::from(e))?;
                    file_paths.push(file_path);
                }
                
                // Handle photo (largest size)
                if let Some(photos) = msg.photo() {
                    if let Some(photo) = photos.last() { // Get the largest photo
                        let file_id = &photo.file.id;
                        let file_info = bot.get_file(file_id).await?;
                        let file_path = format!("/tmp/{}_photo_{}.jpg", user_id, photo.file.id);
                        let mut file = File::create(&file_path).await.map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
                        bot.download_file(&file_info.path, &mut file).await.map_err(|e| teloxide::RequestError::from(e))?;
                        file_paths.push(file_path);
                    }
                }
                
                // Handle video
                if let Some(video) = msg.video() {
                    let file_id = &video.file.id;
                    let file_info = bot.get_file(file_id).await?;
                    let filename = video.file_name.clone().unwrap_or_else(|| "video.mp4".to_string());
                    let file_path = format!("/tmp/{}_{}", user_id, filename);
                    let mut file = File::create(&file_path).await.map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
                    bot.download_file(&file_info.path, &mut file).await.map_err(|e| teloxide::RequestError::from(e))?;
                    file_paths.push(file_path);
                }
                
                // Handle audio
                if let Some(audio) = msg.audio() {
                    let file_id = &audio.file.id;
                    let file_info = bot.get_file(file_id).await?;
                    let filename = audio.file_name.clone().unwrap_or_else(|| "audio.mp3".to_string());
                    let file_path = format!("/tmp/{}_{}", user_id, filename);
                    let mut file = File::create(&file_path).await.map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
                    bot.download_file(&file_info.path, &mut file).await.map_err(|e| teloxide::RequestError::from(e))?;
                    file_paths.push(file_path);
                }
                
                if !file_paths.is_empty() {
                    match upload_files_to_vector_store(user_id, &db, &openai_api_key, file_paths.clone()).await {
                        Ok(vector_store_id) => {
                            let file_count = file_paths.len();
                            let files_text = if file_count == 1 { "file" } else { "files" };
                            bot.send_message(msg.chat.id, format!("✅ {} {} uploaded and indexed! Your vector store ID: {}", file_count, files_text, vector_store_id)).await?;
                        },
                        Err(e) => {
                            bot.send_message(msg.chat.id, format!("[Upload error]: {}", e)).await?;
                        }
                    }
                } else {
                    bot.send_message(msg.chat.id, "No supported files found in your message. Please attach documents, photos, videos, or audio files.").await?;
                }
            }
            respond(())
        }
    });

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
