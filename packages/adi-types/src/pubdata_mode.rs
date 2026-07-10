//! Data-availability pubdata mode.

use clap::ValueEnum;
use serde::{de::Deserializer, Deserialize, Serialize, Serializer};
use strum::{Display, EnumString};

/// How a chain publishes its pubdata (data availability mode).
///
/// This single axis replaces the former `blobs` / `validium` boolean pair. It
/// drives three things:
/// - the on-chain L2 DA commitment scheme set via `setDAValidatorPair`
///   ([`Self::da_commitment_scheme`]),
/// - the server's `l1_sender_pubdata_mode` value ([`Self::server_pubdata_mode`]),
/// - the zkstack `l1_batch_commit_data_generator_mode` at chain creation
///   (rollup vs validium).
///
/// # Serde Behavior
/// - Serializes to snake_case: `"blobs"`, `"calldata"`, `"custom_da"`.
/// - Deserializes case-insensitively, ignoring `-`/`_` (accepts `custom-da`,
///   `customda`, `CustomDA`, ...).
#[derive(Clone, Copy, Debug, Default, Display, EnumString, PartialEq, Eq, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum PubdataMode {
    /// EIP-4844 blobs (zkOS blobs DA). Rollup settling on L1.
    #[default]
    Blobs,
    /// Pubdata posted as L1 calldata. Rollup DA, calldata transport.
    Calldata,
    /// Custom / external DA (e.g. Avail). Only a keccak commitment is on-chain.
    CustomDa,
}

impl PubdataMode {
    /// On-chain `L2DACommitmentScheme` value for `setDAValidatorPair`.
    ///
    /// Matches the server's `DACommitmentScheme`: `BlobsZKsyncOS` = 4,
    /// `BlobsAndPubdataKeccak256` = 3, `PubdataKeccak256` = 2.
    #[must_use]
    pub fn da_commitment_scheme(&self) -> u8 {
        match self {
            Self::Blobs => 4,
            Self::Calldata => 3,
            Self::CustomDa => 2,
        }
    }

    /// The server's `l1_sender_pubdata_mode` string for this mode.
    ///
    /// The server names custom DA `External`.
    #[must_use]
    pub fn server_pubdata_mode(&self) -> &'static str {
        match self {
            Self::Blobs => "Blobs",
            Self::Calldata => "Calldata",
            Self::CustomDa => "External",
        }
    }

    /// zkstack `l1_batch_commit_data_generator_mode` at chain creation.
    ///
    /// Blobs/Calldata are rollups; custom DA (Avail) is created as a validium
    /// and later gets its external DA validator attached.
    #[must_use]
    pub fn zkstack_da_mode(&self) -> &'static str {
        match self {
            Self::Blobs | Self::Calldata => "rollup",
            Self::CustomDa => "validium",
        }
    }
}

impl Serialize for PubdataMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::Blobs => "blobs",
            Self::Calldata => "calldata",
            Self::CustomDa => "custom_da",
        })
    }
}

impl<'de> Deserialize<'de> for PubdataMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Normalize: remove hyphens/underscores and lowercase.
        let normalized = s.to_lowercase().replace(['-', '_'], "");
        match normalized.as_str() {
            "blobs" => Ok(Self::Blobs),
            "calldata" => Ok(Self::Calldata),
            "customda" | "external" => Ok(Self::CustomDa),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["blobs", "calldata", "custom_da"],
            )),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn commitment_scheme_values() {
        assert_eq!(PubdataMode::Blobs.da_commitment_scheme(), 4);
        assert_eq!(PubdataMode::Calldata.da_commitment_scheme(), 3);
        assert_eq!(PubdataMode::CustomDa.da_commitment_scheme(), 2);
    }

    #[test]
    fn server_pubdata_mode_strings() {
        assert_eq!(PubdataMode::Blobs.server_pubdata_mode(), "Blobs");
        assert_eq!(PubdataMode::Calldata.server_pubdata_mode(), "Calldata");
        assert_eq!(PubdataMode::CustomDa.server_pubdata_mode(), "External");
    }

    #[test]
    fn zkstack_da_mode_strings() {
        assert_eq!(PubdataMode::Blobs.zkstack_da_mode(), "rollup");
        assert_eq!(PubdataMode::Calldata.zkstack_da_mode(), "rollup");
        assert_eq!(PubdataMode::CustomDa.zkstack_da_mode(), "validium");
    }

    #[test]
    fn default_is_blobs() {
        assert_eq!(PubdataMode::default(), PubdataMode::Blobs);
    }

    #[test]
    fn deserialize_case_insensitive() {
        assert_eq!(
            serde_yaml::from_str::<PubdataMode>("calldata").unwrap(),
            PubdataMode::Calldata
        );
        assert_eq!(
            serde_yaml::from_str::<PubdataMode>("custom-da").unwrap(),
            PubdataMode::CustomDa
        );
        assert_eq!(
            serde_yaml::from_str::<PubdataMode>("CustomDA").unwrap(),
            PubdataMode::CustomDa
        );
        assert_eq!(
            serde_yaml::from_str::<PubdataMode>("Blobs").unwrap(),
            PubdataMode::Blobs
        );
    }

    #[test]
    fn serialize_roundtrip() {
        for mode in [
            PubdataMode::Blobs,
            PubdataMode::Calldata,
            PubdataMode::CustomDa,
        ] {
            let s = serde_yaml::to_string(&mode).unwrap();
            let back: PubdataMode = serde_yaml::from_str(&s).unwrap();
            assert_eq!(mode, back);
        }
    }

    #[test]
    fn from_str_kebab() {
        assert_eq!(
            "custom-da".parse::<PubdataMode>().unwrap(),
            PubdataMode::CustomDa
        );
        assert_eq!("blobs".parse::<PubdataMode>().unwrap(), PubdataMode::Blobs);
    }
}
