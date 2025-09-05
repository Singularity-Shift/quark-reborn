use crate::dependencies::BotDependencies;
use crate::group::document_library::GroupDocuments;
use crate::user_conversation::dto::FileInfo;
use open_ai_rust_responses_by_sshift::files::FilePurpose;
use open_ai_rust_responses_by_sshift::vector_stores::{
    AddFileToVectorStoreRequest, CreateVectorStoreRequest,
};

pub async fn upload_files_to_group_vector_store(
    group_id: String,
    bot_deps: BotDependencies,
    file_paths: Vec<String>,
) -> Result<String, anyhow::Error> {
    let group_docs = GroupDocuments::new(&bot_deps.db)?;
    let mut file_ids = Vec::new();

    // Check if group has invalid vector store ID and clear stale data upfront
    if let Some(existing_vs_id) = group_docs.get_group_vector_store_id(group_id.clone()) {
        if existing_vs_id.is_empty() || !existing_vs_id.starts_with("vs_") {
            // Clear stale file tracking before adding new files
            group_docs.clear_group_files(group_id.clone())?;
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
        // Store file ID and name in group's local database for reliable tracking
        let filename = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown_file")
            .to_string();
        group_docs.add_group_file(group_id.clone(), &file.id, &filename)?;
    }

    // Check if group already has a vector store
    let vector_store_id = if let Some(existing_vs_id) = group_docs.get_group_vector_store_id(group_id.clone()) {
        // Check if the vector store ID is valid (not empty and starts with 'vs_')
        if existing_vs_id.is_empty() || !existing_vs_id.starts_with("vs_") {
            // Invalid vector store ID, create a new one
            let vs_request = CreateVectorStoreRequest {
                name: format!("group_{}_vector_store", group_id),
                file_ids: file_ids.clone(),
            };

            let new_vector_store = client.vector_stores.create(vs_request).await?;
            let new_vs_id = new_vector_store.id;
            group_docs.set_group_vector_store_id(group_id.clone(), &new_vs_id)?;

            new_vs_id
        } else {
            // Group has existing valid vector store, add files to it
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
                            "Successfully added file {} to existing group vector store {}",
                            file_id,
                            existing_vs_id
                        );
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        // If vector store doesn't exist, clear it and create a new one
                        if error_msg.contains("vector store") && error_msg.contains("not found") {
                            log::warn!(
                                "Vector store {} not found when adding files, creating new vector store for group {}",
                                existing_vs_id,
                                group_id
                            );

                            // Clear the orphaned vector store reference
                            group_docs.set_group_vector_store_id(group_id.clone(), "")?;
                            group_docs.clear_group_files(group_id.clone())?;

                            // Create a new vector store with all files
                            let vs_request = CreateVectorStoreRequest {
                                name: format!("group_{}_vector_store", group_id),
                                file_ids: file_ids.clone(),
                            };

                            let new_vector_store = client.vector_stores.create(vs_request).await?;
                            let new_vs_id = new_vector_store.id;
                            group_docs.set_group_vector_store_id(group_id.clone(), &new_vs_id)?;

                            return Ok(new_vs_id);
                        } else {
                            return Err(e.into());
                        }
                    }
                }
            }

            existing_vs_id
        }
    } else {
        // Group doesn't have a vector store, create a new one
        let vs_request = CreateVectorStoreRequest {
            name: format!("group_{}_vector_store", group_id),
            file_ids: file_ids.clone(),
        };

        let new_vector_store = client.vector_stores.create(vs_request).await?;
        let new_vs_id = new_vector_store.id;
        group_docs.set_group_vector_store_id(group_id.clone(), &new_vs_id)?;

        new_vs_id
    };

    log::info!(
        "Successfully uploaded {} files to group vector store {} for group {}",
        file_ids.len(),
        vector_store_id,
        group_id
    );

    Ok(vector_store_id)
}

/// List files with names from group's local database (reliable, immediate)
/// This bypasses the unreliable OpenAI vector store file listing API
pub fn list_group_files_with_names(
    group_id: String,
    bot_deps: BotDependencies,
) -> Result<Vec<FileInfo>, anyhow::Error> {
    let group_docs = GroupDocuments::new(&bot_deps.db)?;
    let files = group_docs.get_group_files(group_id);
    Ok(files)
}

/// Delete a specific file from a group vector store
/// Note: This only removes the file from the vector store, not from OpenAI's file storage
/// To completely delete the file, you must also call client.files.delete()
pub async fn delete_file_from_group_vector_store(
    group_id: String,
    bot_deps: BotDependencies,
    vector_store_id: &str,
    file_id: &str,
) -> Result<(), anyhow::Error> {
    let client = bot_deps.ai.get_client();
    let group_docs = GroupDocuments::new(&bot_deps.db)?;

    // Remove file from vector store - now returns VectorStoreFileDeleteResponse
    match client
        .vector_stores
        .delete_file(vector_store_id, file_id)
        .await
    {
        Ok(_delete_response) => {
            log::info!(
                "Successfully deleted file {} from group vector store {}",
                file_id,
                vector_store_id
            );
        }
        Err(e) => {
            let error_msg = e.to_string();
            // If vector store doesn't exist, clean up the database references
            if error_msg.contains("vector store") && error_msg.contains("not found") {
                log::warn!(
                    "Vector store {} not found, cleaning up database references for group {}",
                    vector_store_id,
                    group_id
                );
                group_docs.set_group_vector_store_id(group_id.clone(), "")?;
                group_docs.clear_group_files(group_id.clone())?;
                return Err(anyhow::anyhow!(
                    "Your group document library is no longer available. Please upload files again via Group Settings → Document Library → Upload Files to create a new document library."
                ));
            }
            // Re-throw other errors
            return Err(e.into());
        }
    }

    // Remove file ID from local tracking
    group_docs.remove_group_file_id(group_id, file_id)?;

    Ok(())
}

/// Delete an entire group vector store
/// Note: This does not delete the underlying files from OpenAI's file storage
/// The files remain in storage and can be used in other vector stores
pub async fn delete_group_vector_store(
    group_id: String,
    bot_deps: BotDependencies,
) -> Result<(), anyhow::Error> {
    let group_docs = GroupDocuments::new(&bot_deps.db)?;
    let client = bot_deps.ai.get_client();

    // Get the group's vector store ID
    if let Some(vector_store_id) = group_docs.get_group_vector_store_id(group_id.clone()) {
        // Only try to delete if vector store ID is not empty
        if !vector_store_id.is_empty() {
            match client.vector_stores.delete(&vector_store_id).await {
                Ok(_deleted_store) => {
                    log::info!(
                        "Successfully deleted vector store {} for group {}",
                        vector_store_id,
                        group_id
                    );
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    // If vector store doesn't exist, that's fine - we still want to clean up database
                    if error_msg.contains("vector store") && error_msg.contains("not found") {
                        log::warn!(
                            "Vector store {} was already deleted, cleaning up database references for group {}",
                            vector_store_id,
                            group_id
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

        // Clear the vector store ID from group's record
        group_docs.set_group_vector_store_id(group_id.clone(), "")?;

        // Clear all file IDs from local tracking
        group_docs.clear_group_files(group_id)?;
    }

    Ok(())
}
