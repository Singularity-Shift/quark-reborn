use std::env;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::upload::Media;
use google_cloud_storage::http::objects::upload::UploadType;

use crate::common::TargetFile;

pub async fn upload_files(target_files: &[TargetFile]) -> Result<()> {
    println!("🚀 Starting AI files upload/update to Google Cloud Storage...");

    // Required environment variables for bucket and naming
    let project_id = env::var("PROJECT_ID").unwrap_or_default();
    let bucket_name =
        env::var("BUCKET").map_err(|_| anyhow!("BUCKET environment variable not set"))?;

    println!("✅ Environment variables loaded:");
    if !project_id.is_empty() {
        println!("   📁 Project ID: {}", project_id);
    }
    println!("   🪣 Bucket: {}", bucket_name);

    // Create Google Cloud Storage client using Application Default Credentials
    // This respects GOOGLE_APPLICATION_CREDENTIALS or `gcloud auth application-default login`
    let config = ClientConfig::default().with_auth().await?;
    let client = Client::new(config);
    println!("🔗 Google Cloud Storage client created with ADC");

    // Upload/update files to Google Cloud Storage
    let mut uploaded_files = Vec::new();

    for (source_path, filename, _) in target_files {
        println!("📤 Processing file: {}...", source_path);

        if Path::new(source_path).exists() {
            match upload_file_to_storage(&client, &bucket_name, source_path, filename).await {
                Ok(_) => {
                    println!("✅ Successfully processed {} in bucket", filename);
                    uploaded_files.push((filename.to_string(), source_path.to_string()));
                }
                Err(e) => {
                    println!("❌ Failed to process {}: {}", filename, e);
                }
            }
        } else {
            println!("⚠️  Source file not found: {}", source_path);
        }
    }

    println!("📊 Upload/Update Summary:");
    println!("   Total files processed: {}", target_files.len());
    println!("   Successfully processed: {}", uploaded_files.len());
    println!(
        "   Failed operations: {}",
        target_files.len() - uploaded_files.len()
    );

    if !uploaded_files.is_empty() {
        println!("✅ Successfully processed files:");
        for (filename, source_path) in uploaded_files {
            println!("   - {} (from {})", filename, source_path);
        }
    }

    println!("🎉 AI files upload/update to Google Cloud Storage completed!");
    Ok(())
}

async fn upload_file_to_storage(
    client: &Client,
    bucket_name: &str,
    source_path: &str,
    object_name: &str,
) -> Result<()> {
    println!(
        "📤 Uploading {} to bucket {} as {}...",
        source_path, bucket_name, object_name
    );

    // Read the file content
    let file_content = fs::read(source_path)?;
    println!("📖 Read {} bytes from {}", file_content.len(), source_path);

    let media = Media::new(object_name.to_string());

    // Upload the file using google-cloud-storage crate
    // This will create new objects or update existing ones
    let _object = client
        .upload_object(
            &google_cloud_storage::http::objects::upload::UploadObjectRequest {
                bucket: bucket_name.to_string(),
                ..Default::default()
            },
            file_content,
            &UploadType::Simple(media),
        )
        .await?;

    println!(
        "💾 Successfully uploaded/updated {} in bucket {} (Object: {})",
        source_path, bucket_name, object_name
    );
    Ok(())
}
