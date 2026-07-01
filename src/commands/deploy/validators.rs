//! Validator role configuration for deployed chains.

use adi_ecosystem::{add_validator_roles, remove_validator_roles, DeployedContracts};
use adi_state::StateManager;
use adi_types::Operators;
use secrecy::SecretString;

use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Configure validator roles for the deployed chain.
///
/// Revokes default blob_operator roles assigned by zkstack, builds operator
/// addresses from config with wallet fallback, persists them to state,
/// revokes replaced default operators, and assigns final roles.
pub async fn configure_validator_roles(
    context: &Context,
    state_manager: &StateManager,
    chain_name: &str,
    rpc_url: &str,
    deployed: &DeployedContracts,
    governor_key: &SecretString,
    gas_multiplier: Option<u64>,
) -> Result<()> {
    ui::section("Configuring Validator Roles")?;

    let chain_wallets = state_manager
        .chain(chain_name)
        .wallets()
        .await
        .wrap_err("Failed to load chain wallets for operator role assignment")?;

    // Always revoke blob_operator roles (zkstack assigns them by default)
    if let Some(blob_op) = chain_wallets.blob_operator.as_ref() {
        let blob_revoke = Operators {
            blob_operator: Some(blob_op.address),
            ..Default::default()
        };

        let revoke_hashes = remove_validator_roles(
            rpc_url,
            deployed,
            &blob_revoke,
            governor_key,
            gas_multiplier,
            context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to revoke blob_operator validator roles")?;

        if !revoke_hashes.is_empty() {
            ui::success(format!(
                "Revoked blob_operator validator roles: {} transactions",
                ui::green(revoke_hashes.len())
            ))?;
        }
    }

    let config_operators = &context.config().operators;

    let operators = Operators {
        operator: config_operators
            .operator
            .or_else(|| chain_wallets.operator.as_ref().map(|w| w.address)),
        prove_operator: config_operators
            .prove_operator
            .or_else(|| chain_wallets.prove_operator.as_ref().map(|w| w.address)),
        execute_operator: config_operators
            .execute_operator
            .or_else(|| chain_wallets.execute_operator.as_ref().map(|w| w.address)),
        ..Default::default()
    };

    if operators.has_any() {
        let chain_ops = state_manager.chain(chain_name);
        if chain_ops.operators_exist().await.unwrap_or(false) {
            chain_ops
                .update_operators(&operators)
                .await
                .wrap_err("Failed to update operators")?;
        } else {
            chain_ops
                .create_operators(&operators)
                .await
                .wrap_err("Failed to create operators")?;
        }
    }

    if !operators.has_any() {
        ui::warning("No operators configured - skipping validator role assignment")?;
        ui::info("Operators will be assigned from wallets.yaml or via --operator CLI flag")?;
        return Ok(());
    }

    // Revoke default wallet operators being replaced by config addresses
    let wallet_operator = chain_wallets.operator.as_ref().map(|w| w.address);
    let wallet_prove = chain_wallets.prove_operator.as_ref().map(|w| w.address);
    let wallet_execute = chain_wallets.execute_operator.as_ref().map(|w| w.address);

    let operators_to_revoke = Operators {
        operator: if config_operators.operator.is_some()
            && config_operators.operator != wallet_operator
        {
            wallet_operator
        } else {
            None
        },
        prove_operator: if config_operators.prove_operator.is_some()
            && config_operators.prove_operator != wallet_prove
        {
            wallet_prove
        } else {
            None
        },
        execute_operator: if config_operators.execute_operator.is_some()
            && config_operators.execute_operator != wallet_execute
        {
            wallet_execute
        } else {
            None
        },
        ..Default::default()
    };

    if operators_to_revoke.has_any() {
        let revoke_hashes = remove_validator_roles(
            rpc_url,
            deployed,
            &operators_to_revoke,
            governor_key,
            gas_multiplier,
            context.logger().as_ref(),
        )
        .await
        .wrap_err("Failed to revoke default validator roles")?;

        ui::success(format!(
            "Revoked default validator roles: {} transactions",
            ui::green(revoke_hashes.len())
        ))?;
    }

    let tx_hashes = add_validator_roles(
        rpc_url,
        deployed,
        &operators,
        governor_key,
        gas_multiplier,
        context.logger().as_ref(),
    )
    .await
    .wrap_err("Failed to add validator roles")?;

    ui::success(format!(
        "Validator roles configured: {} transactions confirmed",
        ui::green(tx_hashes.len())
    ))?;

    Ok(())
}
