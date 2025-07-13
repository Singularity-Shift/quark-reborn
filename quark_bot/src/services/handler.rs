use anyhow::{Result, anyhow};
use reqwest::Client;

use log::{debug, error, info, warn};
use quark_core::helpers::dto::{
    CreateGroupResponse, Endpoints, PayUsersRequest, PurchaseRequest, TransactionResponse,
};

#[derive(Clone)]
pub struct Services {
    client: Client,
}

impl Services {
    pub fn new() -> Self {
        let client = Client::new();

        Self { client }
    }

    pub async fn pay_users(
        &self,
        token: String,
        request: PayUsersRequest,
    ) -> Result<TransactionResponse> {
        let url = Endpoints::PayUsers.to_string();
        debug!("🌐 Making user service request to: {}", url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request)
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                debug!("📡 Server response status: {}", status);
                debug!("📡 Server response headers: {:?}", resp.headers());

                if resp.status().is_success() {
                    info!("✅ User service call successful - Status: {}", status);
                    let pay_users_response: TransactionResponse = resp.json().await?;
                    Ok(pay_users_response)
                } else {
                    // Get the error response body for detailed error information
                    let error_body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unable to read error body".to_string());

                    error!("❌ Server responded with error status: {}", status);
                    error!("❌ Server error response body: {}", error_body);
                    error!("❌ Request URL: {}", url);
                    error!(
                        "❌ JWT token (first 20 chars): {}...",
                        if token.len() > 20 {
                            &token[..20]
                        } else {
                            &token
                        }
                    );

                    // Provide specific error messages based on status code
                    let error_message = match status.as_u16() {
                        401 => "Authentication failed - JWT token is invalid or expired",
                        403 => "Access forbidden - insufficient permissions",
                        404 => "User service endpoint not found",
                        429 => "Too many requests - rate limit exceeded",
                        500..=599 => "Internal server error - please try again later",
                        _ => "Unknown server error",
                    };

                    warn!("⚠️ {}", error_message);

                    Err(anyhow!(
                        "User service failed with status {}: {}. Server response: {}",
                        status,
                        error_message,
                        error_body
                    ))
                }
            }
            Err(network_error) => {
                error!(
                    "❌ Network error during user service call: {:?}",
                    network_error
                );
                error!("❌ Failed to connect to: {}", url);
                error!("❌ Network error details: {}", network_error);

                // Check for specific network error types
                if network_error.is_timeout() {
                    error!("⏰ Request timed out");
                } else if network_error.is_connect() {
                    error!("🔌 Connection failed - server may be down");
                } else if network_error.is_request() {
                    error!("📝 Request building failed");
                }

                Err(anyhow!("Network error: {}", network_error))
            }
        }
    }

    pub async fn create_group(&self, token: String) -> Result<CreateGroupResponse> {
        let url = Endpoints::CreateGroup.to_string();
        debug!("🌐 Making group service request to: {}", url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                debug!("📡 Server response status: {}", status);
                debug!("📡 Server response headers: {:?}", resp.headers());

                if resp.status().is_success() {
                    info!("✅ Group service call successful - Status: {}", status);
                    let create_group_response: CreateGroupResponse = resp.json().await?;
                    Ok(create_group_response)
                } else {
                    let error_body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unable to read error body".to_string());

                    error!("❌ Server responded with error status: {}", status);
                    error!("❌ Server error response body: {}", error_body);
                    error!("❌ Request URL: {}", url);

                    Err(anyhow!(
                        "Group service failed with status {}: {}",
                        status,
                        error_body
                    ))
                }
            }
            Err(network_error) => {
                error!(
                    "❌ Network error during group service call: {:?}",
                    network_error
                );
                error!("❌ Failed to connect to: {}", url);
                error!("❌ Network error details: {}", network_error);

                Err(anyhow!("Network error: {}", network_error))
            }
        }
    }

    pub async fn pay_members(&self, token: String) -> Result<TransactionResponse> {
        let url = Endpoints::PayMembers.to_string();
        debug!("🌐 Making member service request to: {}", url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                debug!("📡 Server response status: {}", status);
                debug!("📡 Server response headers: {:?}", resp.headers());

                if resp.status().is_success() {
                    info!("✅ Member service call successful - Status: {}", status);
                    let pay_members_response: TransactionResponse = resp.json().await?;
                    Ok(pay_members_response)
                } else {
                    let error_body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unable to read error body".to_string());

                    error!("❌ Server responded with error status: {}", status);
                    error!("❌ Server error response body: {}", error_body);
                    error!("❌ Request URL: {}", url);

                    Err(anyhow!(
                        "Member service failed with status {}: {}",
                        status,
                        error_body
                    ))
                }
            }
            Err(network_error) => {
                error!(
                    "❌ Network error during member service call: {:?}",
                    network_error
                );
                error!("❌ Failed to connect to: {}", url);
                error!("❌ Network error details: {}", network_error);

                Err(anyhow!("Network error: {}", network_error))
            }
        }
    }

    pub async fn purchase(&self, token: String, request: PurchaseRequest) -> Result<()> {
        let url = Endpoints::Purchase.to_string();
        debug!("🌐 Making payment service request to: {}", url);

        println!("request: {:?}", request);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request)
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                debug!("📡 Server response status: {}", status);
                debug!("📡 Server response headers: {:?}", resp.headers());

                if resp.status().is_success() {
                    info!("✅ Payment service call successful - Status: {}", status);
                    let digest = resp.json::<()>().await;

                    if digest.is_err() {
                        error!("❌ Failed to parse payment response: {:?}", digest.err());
                        Err(anyhow!("Failed to parse payment response"))
                    } else {
                        Ok(digest.unwrap())
                    }
                } else {
                    let error_body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unable to read error body".to_string());

                    error!("❌ Server responded with error status: {}", status);
                    error!("❌ Server error response body: {}", error_body);
                    error!("❌ Request URL: {}", url);

                    Err(anyhow!(
                        "Payment service failed with status {}: {}",
                        status,
                        error_body
                    ))
                }
            }
            Err(network_error) => {
                error!(
                    "❌ Network error during payment service call: {:?}",
                    network_error
                );
                error!("❌ Failed to connect to: {}", url);
                error!("❌ Network error details: {}", network_error);

                Err(anyhow!("Network error: {}", network_error))
            }
        }
    }
}
