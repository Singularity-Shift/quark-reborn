use std::env;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::list::ListObjectsRequest;
use google_cloud_storage::http::objects::Object;

use crate::common::TargetFile;

pub async fn download_files(target_files: &[TargetFile]) -> Result<()> {
    println!("🚀 Starting AI files download and replacement from Google Cloud Storage...");

    // Required environment variables for bucket and naming
    let project_id = env::var("PROJECT_ID").unwrap_or_default();
    let bucket_name = env::var("BUCKET")
        .map_err(|_| anyhow!("BUCKET environment variable not set"))?;

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

    // List objects
    println!("🔍 Exploring Google Cloud Storage bucket...");
    let bucket_objects = list_bucket_objects(&client, &bucket_name).await?;
    
    if bucket_objects.is_empty() {
        println!("❌ No files found in the bucket");
        println!("💡 This might mean:");
        println!("   - The bucket is empty");
        println!("   - The bucket name is incorrect");
        println!("   - The credentials don't have access to this bucket");
        return Ok(());
    }

    println!("📁 Files found in bucket:");
    for obj in &bucket_objects {
        println!("   - {} (Size: {} bytes)", obj.name, obj.size);
    }

    // Create temporary directory for downloads
    let temp_dir = "temp_downloads";
    if Path::new(temp_dir).exists() {
        fs::remove_dir_all(temp_dir)?;
    }
    fs::create_dir(temp_dir)?;

    // Change to temp directory
    env::set_current_dir(temp_dir)?;

    // Try to download and map files
    let mut downloaded_files = Vec::new();
    
    for (target_path, default_name, possible_names) in target_files {
        println!("📥 Looking for file to match: {}...", default_name);
        
        // Try to find a matching file
        let matching_object = find_matching_object(&bucket_objects, possible_names);
        
        if let Some(obj) = matching_object {
            println!("🎯 Found matching file: {} -> {}", obj.name, default_name);
            
            match download_file_from_storage(&client, &bucket_name, &obj.name, default_name).await {
                Ok(_) => {
                    println!("✅ Successfully downloaded {}", default_name);
                    downloaded_files.push((default_name.to_string(), target_path.to_string()));
                }
                Err(e) => {
                    println!("❌ Failed to download {}: {}", default_name, e);
                }
            }
        } else {
            println!("⚠️  No matching file found for {}", default_name);
            println!("   Looking for files containing: {:?}", possible_names);
        }
    }

    // Move back to project root
    env::set_current_dir("..")?;

    // Replace files in their locations
    for (filename, target_path) in downloaded_files {
        let source_path = format!("{}/{}", temp_dir, filename);
        let target_path = Path::new(&target_path);

        if Path::new(&source_path).exists() {
            // Ensure target directory exists
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Backup existing file if it exists
            if target_path.exists() {
                let backup_path = format!("{}.backup", target_path.display());
                fs::copy(target_path, &backup_path)?;
                println!("💾 Backed up {} to {}", target_path.display(), backup_path);
            }

            // Replace the file
            fs::copy(&source_path, target_path)?;
            println!("✅ Replaced {} at {}", filename, target_path.display());
        } else {
            println!(
                "❌ Downloaded file {} not found, skipping replacement",
                filename
            );
        }
    }

    // Clean up temporary directory
    fs::remove_dir_all(temp_dir)?;
    println!("🧹 Cleaned up temporary files");

    println!("🎉 AI files download and replacement completed!");
    Ok(())
}

async fn list_bucket_objects(
    client: &Client,
    bucket_name: &str,
) -> Result<Vec<Object>> {
    println!("🔍 Listing objects in bucket: {}", bucket_name);
    
    let request = ListObjectsRequest {
        bucket: bucket_name.to_string(),
        ..Default::default()
    };

    let response = client.list_objects(&request).await?;
    
    if let Some(items) = response.items {
        println!("📊 Found {} objects in bucket", items.len());
        Ok(items)
    } else {
        println!("📊 No objects found in bucket");
        Ok(Vec::new())
    }
}

fn find_matching_object<'a>(
    bucket_objects: &'a [Object],
    possible_names: &[&str],
) -> Option<&'a Object> {
    for obj in bucket_objects {
        let object_name_lower = obj.name.to_lowercase();
        
        for possible_name in possible_names {
            let possible_name_lower = possible_name.to_lowercase();
            
            // Check for exact match
            if object_name_lower == possible_name_lower {
                return Some(obj);
            }
            
            // Check if object name contains the possible name
            if object_name_lower.contains(&possible_name_lower) {
                return Some(obj);
            }
            
            // Check if possible name contains the object name (for partial matches)
            if possible_name_lower.contains(&object_name_lower) {
                return Some(obj);
            }
        }
    }
    None
}

async fn download_file_from_storage(
    client: &Client,
    bucket_name: &str,
    object_name: &str,
    filename: &str,
) -> Result<()> {
    println!("📥 Downloading {} from bucket {}...", object_name, bucket_name);
    
    let request = GetObjectRequest {
        bucket: bucket_name.to_string(),
        object: object_name.to_string(),
        ..Default::default()
    };

    let response = client.download_object(&request, &Range::default()).await?;
    
    // Write to file
    fs::write(filename, &response)?;
    
    println!("💾 Downloaded {} bytes for {}", response.len(), filename);
    Ok(())
}
