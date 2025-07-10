use std::env;
use std::io;
use std::process::Command;

// Docker-optimized system GPG with proper environment setup
fn decrypt_with_system_gpg() -> io::Result<String> {
    // Set environment variables for non-interactive GPG
    unsafe {
        env::set_var("GPG_TTY", "");
        env::set_var("GNUPGHOME", "/tmp/.gnupg");
    }

    let private_key_path =
        env::var("GPG_PRIVATE_KEY").expect("GPG_PRIVATE_KEY environment variable not set");
    let public_key_path =
        env::var("GPG_PUBLIC_KEY").expect("GPG_PUBLIC_KEY environment variable not set");
    let encrypted_reviewer_path =
        env::var("GPG_REVIEWER").expect("GPG_REVIEWER environment variable not set");
    let passphrase =
        env::var("GPG_PASSPHRASE").expect("GPG_PASSPHRASE environment variable not set");

    // Create GPG home directory
    std::fs::create_dir_all("/tmp/.gnupg")?;

    // Set proper permissions for GPG directory
    let _chmod_output = Command::new("chmod").arg("700").arg("/tmp/.gnupg").output();

    // Import the private key
    let import_output = Command::new("gpg")
        .arg("--batch")
        .arg("--yes")
        .arg("--quiet")
        .arg("--pinentry-mode")
        .arg("loopback")
        .arg("--import")
        .arg(private_key_path)
        .env("GPG_TTY", "")
        .env("GNUPGHOME", "/tmp/.gnupg")
        .output()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("GPG not available: {}", e)))?;

    if !import_output.status.success() {
        let stderr = String::from_utf8_lossy(&import_output.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("GPG import failed: {}", stderr),
        ));
    }

    // Import public key if it exists
    let _pub_import_output = Command::new("gpg")
        .arg("--batch")
        .arg("--yes")
        .arg("--quiet")
        .arg("--import")
        .arg(public_key_path)
        .env("GPG_TTY", "")
        .env("GNUPGHOME", "/tmp/.gnupg")
        .output();

    // Decrypt the file
    let decrypt_output = Command::new("gpg")
        .arg("--batch")
        .arg("--yes")
        .arg("--quiet")
        .arg("--trust-model")
        .arg("always")
        .arg("--pinentry-mode")
        .arg("loopback")
        .arg("--passphrase")
        .arg(&passphrase)
        .arg("--decrypt")
        .arg(encrypted_reviewer_path)
        .env("GPG_TTY", "")
        .env("GNUPGHOME", "/tmp/.gnupg")
        .output()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("GPG decrypt failed: {}", e)))?;

    if decrypt_output.status.success() {
        let decrypted_string = String::from_utf8(decrypt_output.stdout).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e))
        })?;
        return Ok(decrypted_string.trim().to_string());
    }

    let stderr = String::from_utf8_lossy(&decrypt_output.stderr);
    Err(io::Error::new(
        io::ErrorKind::Other,
        format!(
            "GPG decryption failed. Make sure GPG_PASSPHRASE environment variable is set correctly: {}",
            stderr
        ),
    ))
}

// GPG decryption function for Docker containers
pub fn decrypt_private_key_in_memory() -> io::Result<String> {
    match decrypt_with_system_gpg() {
        Ok(content) => {
            println!("✅ GPG decryption successful!");
            Ok(content)
        }
        Err(e) => {
            eprintln!("❌ GPG decryption failed: {}", e);
            Err(e)
        }
    }
}
