//! Print surfaced bundles (Safe JSON / calldata) and wait for the operator.

use adi_upgrade::v31::BundleInfo;

use super::super::args::UpgradeArgs;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Print each bundle (Safe JSON and/or calldata) plus who must execute it, and
/// wait for confirmation. No-op in broadcast mode (bundles is empty).
pub(super) fn surface(
    bundles: &[BundleInfo],
    args: &UpgradeArgs,
    l1_chain_id: u64,
    localhost: bool,
) -> Result<()> {
    for b in bundles {
        ui::section(format!("Execute from {}: {}", b.target, b.label))?;
        ui::info(format!(
            "{} transaction(s), signer/executor: {}",
            b.txs.len(),
            b.target
        ))?;
        if args.safe {
            // protocol_ops already wrote each bundle's Safe file; print its path
            // (import it into the Safe Tx Builder) rather than dumping the JSON.
            match &b.safe_json_path {
                Some(path) => {
                    ui::info(format!("Safe Transaction Builder JSON: {}", path.display()))?;
                }
                None => {
                    let json = b.safe_json(l1_chain_id).wrap_err("render Safe JSON")?;
                    println!("\n--- Safe Transaction Builder JSON ---\n{json}\n");
                }
            }
        }
        if args.calldata {
            // On localhost the operator impersonates the owner (anvil --auto-impersonate);
            // on a real chain never emit --unlocked, force a real signing key so an
            // impersonation flag can't be pasted onto a live node by accident.
            let signer = if localhost {
                format!("--from {} --unlocked", b.target)
            } else {
                format!("--private-key <PRIVATE_KEY_OF_{}>", b.target)
            };
            // One ready-to-run cast per tx, numbered and blank-line separated so
            // it is clear where one command ends and the next begins.
            let n = b.txs.len();
            for (i, tx) in b.txs.iter().enumerate() {
                println!("\n# {}/{} — from {}", i + 1, n, b.target);
                let value = if tx.value.is_zero() {
                    String::new()
                } else {
                    format!(" --value {}", tx.value)
                };
                println!(
                    "cast send {} {}{value} {signer} --rpc-url $RPC",
                    tx.to, tx.data
                );
            }
            println!();
        }
        if !args.yes {
            let ok = ui::confirm(format!("Confirm you executed this from {}", b.target))
                .interact()
                .wrap_err("confirmation prompt")?;
            eyre::ensure!(ok, "aborted: operation not confirmed");
        }
    }
    Ok(())
}
