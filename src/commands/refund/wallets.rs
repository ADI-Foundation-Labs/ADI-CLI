//! Wallet collection helpers for the refund command.

use adi_funding::{RefundEntry, WalletRole, WalletSource};
use adi_types::{Wallet, Wallets};

/// Collect refund entries from a Wallets struct.
pub fn collect_wallet_entries(
    wallets: &Wallets,
    source: WalletSource,
    chain_name: Option<&str>,
    out: &mut Vec<RefundEntry>,
) {
    let pairs: &[(WalletRole, &Option<Wallet>)] = &[
        (WalletRole::Deployer, &wallets.deployer),
        (WalletRole::Governor, &wallets.governor),
        (WalletRole::Operator, &wallets.operator),
        (WalletRole::BlobOperator, &wallets.blob_operator),
        (WalletRole::ProveOperator, &wallets.prove_operator),
        (WalletRole::ExecuteOperator, &wallets.execute_operator),
        (WalletRole::FeeAccount, &wallets.fee_account),
        (
            WalletRole::TokenMultiplierSetter,
            &wallets.token_multiplier_setter,
        ),
    ];

    for (role, wallet_opt) in pairs {
        if let Some(wallet) = wallet_opt {
            out.push(RefundEntry {
                role: *role,
                source,
                chain_name: chain_name.map(String::from),
                address: wallet.address,
                private_key: wallet.private_key.clone(),
            });
        }
    }
}

/// Count present wallets in a Wallets struct.
pub fn count_wallets(wallets: &Wallets) -> usize {
    [
        &wallets.deployer,
        &wallets.governor,
        &wallets.operator,
        &wallets.blob_operator,
        &wallets.prove_operator,
        &wallets.execute_operator,
        &wallets.fee_account,
        &wallets.token_multiplier_setter,
    ]
    .iter()
    .filter(|w| w.is_some())
    .count()
}
