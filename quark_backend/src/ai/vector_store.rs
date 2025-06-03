use open_ai_rust_responses_by_sshift::{Client as OAIClient};
use open_ai_rust_responses_by_sshift::files::FilePurpose;
use open_ai_rust_responses_by_sshift::vector_stores::{AddFileToVectorStoreRequest, CreateVectorStoreRequest};
use sled::Db;
use crate::db::UserConversations;

pub async fn upload_files_to_vector_store(
    user_id: i64,
    db: &Db,
    openai_api_key: &str,
    file_paths: Vec<String>,
) -> Result<String, anyhow::Error> {
    let user_convos = UserConversations::new(db)?;
    let client = OAIClient::new(openai_api_key)?;
    let mut file_ids = Vec::new();

    // Upload each file to OpenAI
    for path in &file_paths {
        let file = client
            .files
            .upload_file(path, FilePurpose::Assistants, None)
            .await?;
        file_ids.push(file.id);
    }

    // Create a new vector store with the uploaded files
    let vs_request = CreateVectorStoreRequest {
        name: format!("user_{}_vector_store", user_id),
        file_ids: file_ids.clone(),
    };
    let vector_store = client.vector_stores.create(vs_request).await?;
    let vector_store_id = vector_store.id.clone();

    // (Optional) Add files to vector store again for demonstration, but not needed if already added
    for file_id in &file_ids {
        let add_file_request = AddFileToVectorStoreRequest {
            file_id: file_id.clone(),
            attributes: None,
        };
        let _ = client
            .vector_stores
            .add_file(&vector_store_id, add_file_request)
            .await;
    }

    // Store the vector_store_id in the user's db record
    user_convos.set_vector_store_id(user_id, &vector_store_id)?;

    Ok(vector_store_id)
} 