use crate::ai::handler::AI;
use crate::user_conversation::{dto::FileInfo, handler::UserConversations};
use open_ai_rust_responses_by_sshift::files::FilePurpose;
use open_ai_rust_responses_by_sshift::vector_stores::{
    AddFileToVectorStoreRequest, CreateVectorStoreRequest,
};
use open_ai_rust_responses_by_sshift::Client as OAIClient;
use sled::Db;

pub async fn upload_files_to_vector_store(
    user_id: i64,
    db: &Db,
    ai: AI,
    file_paths: Vec<String>,
) -> Result<String, anyhow::Error> {
    let user_convos = UserConversations::new(db)?;
    let mut file_ids = Vec::new();

    // Check if user has invalid vector store ID and clear stale data upfront
    if let Some(existing_vs_id) = user_convos.get_vector_store_id(user_id) {
        if existing_vs_id.is_empty() || !existing_vs_id.starts_with("vs_") {
            // Clear stale file tracking before adding new files
            user_convos.clear_files(user_id)?;
        }
    }

    let client = ai.get_client();

    // Upload each file to OpenAI
    for path in &file_paths {
        let file = client
            .files
            .upload_file(path, FilePurpose::Assistants, None)
            .await?;
        file_ids.push(file.id.clone());
        // Store file ID and name in user's local database for reliable tracking
        let filename = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown_file")
            .to_string();
        user_convos.add_file(user_id, &file.id, &filename)?;
    }

    // Check if user already has a vector store
    let vector_store_id = if let Some(existing_vs_id) = user_convos.get_vector_store_id(user_id) {
        // Check if the vector store ID is valid (not empty and starts with 'vs_')
        if existing_vs_id.is_empty() || !existing_vs_id.starts_with("vs_") {
            // Invalid vector store ID, create a new one
            let vs_request = CreateVectorStoreRequest {
                name: format!("user_{}_vector_store", user_id),
                file_ids: file_ids.clone(),
            };
            let vector_store = client.vector_stores.create(vs_request).await?;
            let new_vs_id = vector_store.id.clone();

            // Store the new vector_store_id in the user's db record
            user_convos.set_vector_store_id(user_id, &new_vs_id)?;

            new_vs_id
        } else {
            // User has existing valid vector store, add files to it
            for file_id in &file_ids {
                let add_file_request = AddFileToVectorStoreRequest {
                    file_id: file_id.clone(),
                    attributes: None,
                };
                let _ = client
                    .vector_stores
                    .add_file(&existing_vs_id, add_file_request)
                    .await?;
            }
            existing_vs_id
        }
    } else {
        // User doesn't have a vector store, create a new one
        let vs_request = CreateVectorStoreRequest {
            name: format!("user_{}_vector_store", user_id),
            file_ids: file_ids.clone(),
        };
        let vector_store = client.vector_stores.create(vs_request).await?;
        let new_vs_id = vector_store.id.clone();

        // Store the new vector_store_id in the user's db record
        user_convos.set_vector_store_id(user_id, &new_vs_id)?;

        new_vs_id
    };

    Ok(vector_store_id)
}

/// List files with names from user's local database (reliable, immediate)
/// This bypasses the unreliable OpenAI vector store file listing API
pub fn list_user_files_with_names(user_id: i64, db: &Db) -> Result<Vec<FileInfo>, anyhow::Error> {
    let user_convos = UserConversations::new(db)?;
    let files = user_convos.get_files(user_id);
    Ok(files)
}

/// List file IDs only from user's local database (for backward compatibility)
pub fn list_user_files_local(user_id: i64, db: &Db) -> Result<Vec<String>, anyhow::Error> {
    let user_convos = UserConversations::new(db)?;
    let file_ids = user_convos.get_file_ids(user_id);
    Ok(file_ids)
}

/// List all files in a vector store
/// Note: The vector store get() API often returns file_ids: None even when files exist
/// This is a known OpenAI API behavior - use this function to actually list files
pub async fn list_vector_store_files(
    vector_store_id: &str,
    openai_api_key: &str,
) -> Result<Vec<String>, anyhow::Error> {
    let client = OAIClient::new(openai_api_key)?;

    // List files in the vector store using the files API
    let vector_store = client.vector_stores.get(vector_store_id).await?;

    // If file_ids is available in the vector store response, use it
    let file_ids = vector_store.file_ids.unwrap_or_else(Vec::new);

    Ok(file_ids)
}

/// Delete a specific file from a vector store
/// Note: This only removes the file from the vector store, not from OpenAI's file storage
/// To completely delete the file, you must also call client.files.delete()
pub async fn delete_file_from_vector_store(
    user_id: i64,
    db: &Db,
    vector_store_id: &str,
    file_id: &str,
    ai: &AI,
) -> Result<(), anyhow::Error> {
    let client = ai.get_client();
    let user_convos = UserConversations::new(db)?;

    // Remove file from vector store - now returns VectorStoreFileDeleteResponse
    let _delete_response = client
        .vector_stores
        .delete_file(vector_store_id, file_id)
        .await?;

    // Remove from local tracking
    user_convos.remove_file_id(user_id, file_id)?;

    Ok(())
}

/// Delete an entire vector store
/// Note: This does not delete the underlying files from OpenAI's file storage
/// The files remain in storage and can be used in other vector stores
pub async fn delete_vector_store(user_id: i64, db: &Db, ai: &AI) -> Result<(), anyhow::Error> {
    let user_convos = UserConversations::new(db)?;
    let client = ai.get_client();

    // Get the user's vector store ID
    if let Some(vector_store_id) = user_convos.get_vector_store_id(user_id) {
        // Delete the vector store
        let _deleted_store = client.vector_stores.delete(&vector_store_id).await?;

        // Clear the vector store ID from user's record
        user_convos.set_vector_store_id(user_id, "")?;

        // Clear all file IDs from local tracking
        user_convos.clear_files(user_id)?;
    }

    Ok(())
}

/// Delete all files from a vector store and optionally delete the files completely
pub async fn delete_all_files_from_vector_store(
    vector_store_id: &str,
    openai_api_key: &str,
    also_delete_files: bool,
) -> Result<u32, anyhow::Error> {
    let client = OAIClient::new(openai_api_key)?;
    let mut deleted_count = 0;

    // Get vector store and its file IDs
    let vector_store = client.vector_stores.get(vector_store_id).await?;

    let file_ids = vector_store.file_ids.unwrap_or_else(Vec::new);

    for file_id in file_ids {
        // Remove from vector store - now returns VectorStoreFileDeleteResponse
        let _delete_response = client
            .vector_stores
            .delete_file(vector_store_id, &file_id)
            .await?;

        // Optionally delete the file completely from OpenAI storage
        if also_delete_files {
            let _deleted = client.files.delete(&file_id).await?;
        }

        deleted_count += 1;
    }

    Ok(deleted_count)
}
