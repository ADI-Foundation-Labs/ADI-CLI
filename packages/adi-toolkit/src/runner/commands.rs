//! Toolkit command methods (zkstack, forge, cast).

use super::params::RunCommandParams;
use super::ToolkitRunner;
use crate::error::Result;
use adi_docker::transform_url_for_container;
use semver::Version;
use std::path::Path;

impl ToolkitRunner {
    /// Execute zkstack CLI command in toolkit container.
    ///
    /// # Arguments
    /// * `args` - Arguments to pass to zkstack
    /// * `state_dir` - Container working directory (mounted as /workspace)
    /// * `log_dir` - Directory for saving logs (use state_dir if same, or real state dir if using temp)
    /// * `protocol_version` - Protocol version for toolkit image selection
    pub async fn run_zkstack(
        &self,
        args: &[&str],
        state_dir: &Path,
        log_dir: &Path,
        protocol_version: &Version,
    ) -> Result<i64> {
        self.logger
            .debug(&format!("Running zkstack with args: {:?}", args));

        let mut command = vec!["zkstack"];
        command.extend(args);
        let label = format!("Running zkstack {}...", args.first().unwrap_or(&""));

        self.run_command_internal(RunCommandParams {
            command: &command,
            state_dir,
            log_dir,
            protocol_version,
            env_vars: &[],
            log_command: "zkstack",
            log_label: &label,
            quiet: false,
        })
        .await
    }

    /// Execute forge command in toolkit container.
    pub async fn run_forge(
        &self,
        args: &[&str],
        state_dir: &Path,
        protocol_version: &Version,
    ) -> Result<i64> {
        self.logger
            .debug(&format!("Running forge with args: {:?}", args));
        let mut command = vec!["forge"];
        command.extend(args);

        self.run_command(
            &command,
            state_dir,
            protocol_version,
            &[],
            "forge",
            "Running forge...",
        )
        .await
    }

    /// Execute cast command in toolkit container.
    pub async fn run_cast(&self, args: &[&str], protocol_version: &Version) -> Result<i64> {
        self.logger
            .debug(&format!("Running cast with args: {:?}", args));
        let mut command = vec!["cast"];
        command.extend(args);

        let temp_dir = std::env::temp_dir();
        self.logger.debug(&format!(
            "Using temp directory for cast: {}",
            temp_dir.display()
        ));

        self.run_command(
            &command,
            &temp_dir,
            protocol_version,
            &[],
            "cast",
            "Running cast...",
        )
        .await
    }

    /// Execute `forge verify-contract` in toolkit container.
    pub async fn run_forge_verify(
        &self,
        params: &super::params::ForgeVerifyParams<'_>,
    ) -> Result<i64> {
        self.logger.debug(&format!(
            "Running forge verify-contract for {} (contract: {}, root: {})",
            params.address, params.contract_path, params.root_path
        ));

        // Build the forge verify-contract command
        // Forge verify doesn't use src setting, so we prepend contracts/ to the path
        // Exception: lib/ paths (e.g., OpenZeppelin contracts) are at project root level
        let full_contract_path = if params.contract_path.starts_with("lib/") {
            params.contract_path.to_string()
        } else {
            format!("contracts/{}", params.contract_path)
        };
        let chain_id_str = params.chain_id.to_string();
        let mut args: Vec<&str> = vec![
            "forge",
            "verify-contract",
            params.address,
            &full_contract_path,
            "--chain-id",
            &chain_id_str,
            "--verifier",
            params.verifier,
            "--verifier-url",
            params.verifier_url,
            "--root",
            params.root_path,
            "--compiler-version",
            "0.8.28",
            "--evm-version",
            "cancun",
            "--num-of-optimizations",
            "28000",
            "--watch", // Wait for verification to complete (not just submission accepted)
        ];

        if let Some(key) = params.api_key {
            args.push("--etherscan-api-key");
            args.push(key);
        }

        if let Some(ctor_args) = params.constructor_args {
            args.push("--constructor-args");
            args.push(ctor_args);
        }

        let temp_dir = std::env::temp_dir();

        // Run in quiet mode - output is suppressed during batch verification
        // (progress bar shows status, logs are saved to file)
        self.run_command_internal(RunCommandParams {
            command: &args,
            state_dir: &temp_dir,
            log_dir: params.log_dir,
            protocol_version: params.protocol_version,
            env_vars: &[],
            log_command: "forge-verify",
            log_label: &format!("Verifying {}...", params.address),
            quiet: true,
        })
        .await
    }

    /// Execute `zkstack ecosystem init` with foundry.toml permission fix.
    pub async fn run_zkstack_ecosystem_init(
        &self,
        params: &super::params::EcosystemInitParams<'_>,
    ) -> Result<i64> {
        self.logger.debug(&format!(
            "Running zkstack ecosystem init (ecosystem_dir: {}, rpc: {}, deploy_ecosystem: {})",
            params.ecosystem_dir.display(),
            params.l1_rpc_url,
            params.deploy_ecosystem
        ));

        let foundry_fix = r#"sed -i.bak 's/{ access = "read", path = "\.\.\/l1-contracts\/script-out\/" }/{ access = "read-write", path = "..\/l1-contracts\/script-out\/" }/' /deps/zksync-era/contracts/l1-contracts/foundry.toml"#;

        let escaped_chain_name = super::shell_escape(params.chain_name);
        let mut zkstack_args = format!(
            "zkstack ecosystem init \
             --verbose \
             --zksync-os \
             --ignore-prerequisites \
             --observability false \
             --deploy-ecosystem {} \
             --deploy-erc20 false \
             --deploy-paymaster false \
             --chain {}",
            params.deploy_ecosystem, escaped_chain_name
        );

        // When not deploying ecosystem, point to existing contracts config
        if !params.deploy_ecosystem {
            // In container, ecosystem_dir is mounted as /workspace
            // zkstack expects path to ecosystem root where configs/contracts.yaml exists
            zkstack_args.push_str(" --ecosystem-contracts-path /workspace/configs/contracts.yaml");
        }

        if let Some(gas_price) = params.gas_price_wei {
            zkstack_args.push_str(&format!(" -a --with-gas-price -a {}", gas_price));
        }

        let container_rpc_url =
            transform_url_for_container(params.l1_rpc_url, self.logger.as_ref());
        let escaped_rpc_url = super::shell_escape(&container_rpc_url);
        zkstack_args.push_str(&format!(" --l1-rpc-url {}", escaped_rpc_url));

        let shell_cmd = format!(
            r#"{foundry_fix} && \
stdbuf -oL expect -c 'set timeout 3600
log_user 1
spawn {zkstack}
while 1 {{
    expect {{
        eof {{ break }}
        timeout {{ break }}
        -re "\\(.*\\)\\s*$" {{ send "\r" }}
    }}
}}
catch wait result
exit [lindex $result 3]'"#,
            foundry_fix = foundry_fix,
            zkstack = zkstack_args
        );

        let deploy_msg = if params.deploy_ecosystem {
            "deploying ecosystem + chain contracts"
        } else {
            "deploying chain contracts only"
        };
        self.logger.debug(&format!(
            "Fixing foundry.toml permissions and {}",
            deploy_msg
        ));

        let shell_command = vec!["sh", "-c", &shell_cmd];

        let label = if params.deploy_ecosystem {
            "Deploying ecosystem contracts..."
        } else {
            "Deploying chain contracts..."
        };

        self.run_command(
            &shell_command,
            params.ecosystem_dir,
            params.protocol_version,
            &[],
            "deploy",
            label,
        )
        .await
    }
}
