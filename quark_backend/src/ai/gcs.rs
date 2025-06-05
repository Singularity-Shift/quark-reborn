use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use cloud_storage::Client;
use uuid::Uuid;
use std::env;

pub struct GcsImageUploader {
    client: Client,
    bucket_name: String,
}

impl GcsImageUploader {
    pub async fn new(credentials_base64: &str, bucket_name: String) -> Result<Self> {
        // Decode base64 to get the JSON string
        let credentials_json = general_purpose::STANDARD.decode(credentials_base64)?;
        let credentials_str = String::from_utf8(credentials_json)?;
        
        // Set the service account JSON as environment variable for cloud-storage crate
        // This is safe in our context as we're setting a known credential value
        unsafe {
            env::set_var("SERVICE_ACCOUNT_JSON", credentials_str);
        }
        
        // Create client - the cloud-storage crate reads from environment variables
        let client = Client::default();
        
        Ok(Self {
            client,
            bucket_name,
        })
    }
    
    pub async fn upload_base64_image(
        &self,
        base64_data: &str,
        image_format: &str, // "png", "jpg", "webp"
    ) -> Result<String> {
        // Decode base64 to bytes
        let image_bytes = general_purpose::STANDARD.decode(base64_data)?;
        
        // Generate unique filename in the quark folder
        let filename = format!("quark/images/{}.{}", Uuid::new_v4(), image_format);
        
        // Determine content type
        let content_type = match image_format {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg", 
            "webp" => "image/webp",
            _ => "image/png", // default fallback
        };
        
        // Upload the image using cloud-storage crate
        let _object = self.client
            .object()
            .create(&self.bucket_name, image_bytes, &filename, content_type)
            .await?;
        
        // Return public URL (Google Cloud Storage public URL format)
        let public_url = format!(
            "https://storage.googleapis.com/{}/{}",
            self.bucket_name, filename
        );
        
        Ok(public_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // Requires actual GCS credentials
    async fn test_gcs_uploader_creation() {
        // Test with base64 encoded mock credentials
        let mock_credentials_json = r#"{
            "type": "service_account",
            "project_id": "test-project",
            "private_key_id": "test-key-id", 
            "private_key": "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n",
            "client_email": "test@test-project.iam.gserviceaccount.com",
            "client_id": "123456789",
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token"
        }"#;
        
        let mock_credentials_base64 = general_purpose::STANDARD.encode(mock_credentials_json.as_bytes());
        
        // This will fail without real credentials, but tests the structure
        let result = GcsImageUploader::new(&mock_credentials_base64, "test-bucket".to_string()).await;
        // Just test that we can parse the structure without panicking
        assert!(result.is_ok()); // Should work with the cloud-storage crate approach
    }
} 