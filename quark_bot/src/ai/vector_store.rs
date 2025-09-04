use crate::dependencies::BotDependencies;
use crate::user_conversation::{dto::FileInfo, handler::UserConversations};
use open_ai_rust_responses_by_sshift::files::FilePurpose;
use open_ai_rust_responses_by_sshift::vector_stores::{
    AddFileToVectorStoreRequest, CreateVectorStoreRequest,
};

pub async fn upload_files_to_vector_store(
    user_id: i64,
    bot_deps: BotDependencies,
    file_paths: Vec<String>,
) -> Result<String, anyhow::Error> {
    let user_convos = UserConversations::new(&bot_deps.db)?;
    let mut file_ids = Vec::new();

    // Check if user has invalid vector store ID and clear stale data upfront
    if let Some(existing_vs_id) = user_convos.get_vector_store_id(user_id) {
        if existing_vs_id.is_empty() || !existing_vs_id.starts_with("vs_") {
            // Clear stale file tracking before adding new files
            user_convos.clear_files(user_id)?;
        }
    }

    let client = bot_deps.ai.get_client();

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
                match client
                    .vector_stores
                    .add_file(&existing_vs_id, add_file_request)
                    .await
                {
                    Ok(_) => {
                        log::info!(
                            "Successfully added file {} to existing vector store {}",
                            file_id,
                            existing_vs_id
                        );
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        // If vector store doesn't exist, clear it and create a new one
                        if error_msg.contains("vector store") && error_msg.contains("not found") {
                            log::warn!(
                                "Vector store {} not found when adding files, creating new vector store for user {}",
                                existing_vs_id,
                                user_id
                            );

                            // Clear the orphaned vector store reference
                            user_convos.set_vector_store_id(user_id, "")?;
                            user_convos.clear_files(user_id)?;

                            // Create a new vector store with all files
                            let vs_request = CreateVectorStoreRequest {
                                name: format!("user_{}_vector_store", user_id),
                                file_ids: file_ids.clone(),
                            };
                            let vector_store = client.vector_stores.create(vs_request).await?;
                            let new_vs_id = vector_store.id.clone();

                            // Store the new vector_store_id in the user's db record
                            user_convos.set_vector_store_id(user_id, &new_vs_id)?;

                            return Ok(new_vs_id);
                        } else {
                            // Re-throw other errors
                            return Err(e.into());
                        }
                    }
                }
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
pub fn list_user_files_with_names(
    user_id: i64,
    bot_deps: BotDependencies,
) -> Result<Vec<FileInfo>, anyhow::Error> {
    let user_convos = UserConversations::new(&bot_deps.db)?;
    let files = user_convos.get_files(user_id);
    Ok(files)
}

/// Delete a specific file from a vector store
/// Note: This only removes the file from the vector store, not from OpenAI's file storage
/// To completely delete the file, you must also call client.files.delete()
pub async fn delete_file_from_vector_store(
    user_id: i64,
    bot_deps: BotDependencies,
    vector_store_id: &str,
    file_id: &str,
) -> Result<(), anyhow::Error> {
    let client = bot_deps.ai.get_client();
    let user_convos = UserConversations::new(&bot_deps.db)?;

    // Remove file from vector store - now returns VectorStoreFileDeleteResponse
    match client
        .vector_stores
        .delete_file(vector_store_id, file_id)
        .await
    {
        Ok(_delete_response) => {
            log::info!(
                "Successfully deleted file {} from vector store {}",
                file_id,
                vector_store_id
            );
        }
        Err(e) => {
            let error_msg = e.to_string();
            // If vector store doesn't exist, clean up the database references
            if error_msg.contains("vector store") && error_msg.contains("not found") {
                log::warn!(
                    "Vector store {} not found, cleaning up database references for user {}",
                    vector_store_id,
                    user_id
                );
                user_convos.set_vector_store_id(user_id, "")?;
                user_convos.clear_files(user_id)?;
                return Err(anyhow::anyhow!(
                    "Your document library is no longer available. Please upload files again via /usersettings → Document Library → Upload Files to create a new document library."
                ));
            }
            // Re-throw other errors
            return Err(e.into());
        }
    }

    // Remove from local tracking
    user_convos.remove_file_id(user_id, file_id)?;

    Ok(())
}

/// Delete an entire vector store
/// Note: This does not delete the underlying files from OpenAI's file storage
/// The files remain in storage and can be used in other vector stores
pub async fn delete_vector_store(
    user_id: i64,
    bot_deps: BotDependencies,
) -> Result<(), anyhow::Error> {
    let user_convos = UserConversations::new(&bot_deps.db)?;
    let client = bot_deps.ai.get_client();

    // Get the user's vector store ID
    if let Some(vector_store_id) = user_convos.get_vector_store_id(user_id) {
        // Only try to delete if vector store ID is not empty
        if !vector_store_id.is_empty() {
            match client.vector_stores.delete(&vector_store_id).await {
                Ok(_deleted_store) => {
                    log::info!(
                        "Successfully deleted vector store {} for user {}",
                        vector_store_id,
                        user_id
                    );
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    // If vector store doesn't exist, that's fine - we still want to clean up database
                    if error_msg.contains("vector store") && error_msg.contains("not found") {
                        log::warn!(
                            "Vector store {} was already deleted, cleaning up database references for user {}",
                            vector_store_id,
                            user_id
                        );
                    } else {
                        // For other errors, log but continue with cleanup
                        log::error!(
                            "Failed to delete vector store {}: {}, but continuing with database cleanup",
                            vector_store_id,
                            error_msg
                        );
                    }
                }
            }
        }

        // Clear the vector store ID from user's record
        user_convos.set_vector_store_id(user_id, "")?;

        // Clear all file IDs from local tracking
        user_convos.clear_files(user_id)?;
    }

    Ok(())
}
