//! Upload server parameters to HashiCorp Vault.

use std::collections::HashMap;

use adi_vault::VaultClient;
use secrecy::ExposeSecret;

use crate::commands::server_params::ServerParamsArgs;
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Default Vault base URL.
const DEFAULT_VAULT_URL: &str = "https://vault.dev.internal.adifoundation.ai";

/// Upload server parameters to HashiCorp Vault.
pub async fn run(
    args: &ServerParamsArgs,
    context: &Context,
    json_output: &HashMap<&str, Option<String>>,
) -> Result<()> {
    // Confirm upload
    let confirm = cliclack::confirm("Upload these parameters to Vault?")
        .interact()
        .wrap_err("Failed to read confirmation")?;

    if !confirm {
        ui::info("Upload cancelled.")?;
        return Ok(());
    }

    // Resolve vault path
    let vault_path = if let Some(path) = &args.vault_path {
        validate_vault_path(path)?;
        path.clone()
    } else {
        prompt_vault_path()?
    };

    // Create Vault client
    let vault_url = context
        .config()
        .vault
        .api_url
        .as_ref()
        .map(|u| u.as_str().trim_end_matches('/').to_string())
        .unwrap_or_else(|| DEFAULT_VAULT_URL.to_string());

    let client = VaultClient::new(&vault_url).wrap_err("Failed to create Vault client")?;

    // Health check
    let spinner = cliclack::spinner();
    spinner.start("Checking Vault connectivity...");
    client
        .health_check()
        .await
        .wrap_err("Vault health check failed")?;
    spinner.stop("Vault connectivity verified.");

    // Prompt for token
    ui::info(format!(
        "Create a short-lived token in the Vault UI:\n  {vault_url}"
    ))?;

    let token = args
        .vault_token
        .as_ref()
        .map(|t| Ok(t.expose_secret().to_string()))
        .unwrap_or_else(|| {
            cliclack::password("Vault token:")
                .interact()
                .wrap_err("Failed to read vault token")
        })?;

    // Write secret
    let data =
        serde_json::to_value(json_output).wrap_err("Failed to serialize parameters to JSON")?;

    let spinner = cliclack::spinner();
    spinner.start("Uploading secret...");
    client
        .write_secret(&token, &vault_path, &data)
        .await
        .wrap_err("Failed to write secret to Vault")?;
    spinner.stop("Secret written successfully.");

    Ok(())
}

/// Validate a Vault API path starts with `/v1/`.
fn validate_vault_path(path: &str) -> Result<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(eyre::eyre!("Vault path must not be empty."));
    }
    if !trimmed.starts_with("/v1/") {
        return Err(eyre::eyre!("Vault path must start with /v1/."));
    }
    Ok(())
}

/// Prompt user for Vault secret path with validation.
fn prompt_vault_path() -> Result<String> {
    let path: String = cliclack::input(
        "Vault secret path (e.g. /v1/Adi-chain/data/Adi-chain/adi/devnet1/server):",
    )
    .validate(|input: &String| {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            Err("Path must not be empty.")
        } else if !trimmed.starts_with("/v1/") {
            Err("Path must start with /v1/.")
        } else {
            Ok(())
        }
    })
    .interact()
    .wrap_err("Failed to read vault path")?;
    Ok(path.trim().to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn validate_vault_path_accepts_valid() {
        assert!(validate_vault_path("/v1/Adi-chain/data/server").is_ok());
    }

    #[test]
    fn validate_vault_path_rejects_empty() {
        let err = validate_vault_path("").unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn validate_vault_path_rejects_whitespace_only() {
        let err = validate_vault_path("   ").unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn validate_vault_path_rejects_no_v1_prefix() {
        let err = validate_vault_path("/secret/data/foo").unwrap_err();
        assert!(err.to_string().contains("must start with /v1/"));
    }
}
