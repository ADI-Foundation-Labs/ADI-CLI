//! Diamond cut data parser for extracting facet addresses.
//!
//! Parses the ABI-encoded `diamond_cut_data` field from ecosystem contracts
//! to extract individual facet and DiamondInit addresses.

use alloy_primitives::{Address, Bytes};
use alloy_sol_types::{sol, SolType};

// Define the Solidity types for ABI decoding
sol! {
    /// Individual facet cut entry in diamond cut data.
    struct FacetCut {
        address facetAddress;
        uint8 action;
        bool isFreezable;
        bytes4[] selectors;
    }

    /// Diamond cut data structure containing all facets and init address.
    struct DiamondCutData {
        FacetCut[] facetCuts;
        address initAddress;
        bytes initCalldata;
    }
}

/// Parsed facet addresses from diamond cut data.
#[derive(Debug, Clone, Default)]
pub struct DiamondFacets {
    /// Admin facet address.
    pub admin_facet: Option<Address>,
    /// Executor facet address.
    pub executor_facet: Option<Address>,
    /// Mailbox facet address.
    pub mailbox_facet: Option<Address>,
    /// Getters facet address.
    pub getters_facet: Option<Address>,
    /// DiamondInit contract address.
    pub diamond_init: Option<Address>,
}

/// Parse diamond_cut_data hex string to extract facet addresses.
///
/// The diamond_cut_data is ABI-encoded as a tuple containing:
/// - `FacetCut[]`: Array of facet cuts with addresses and selectors
/// - `address`: DiamondInit contract address
/// - `bytes`: Init calldata
///
/// # Arguments
///
/// * `hex_data` - Hex-encoded diamond cut data (with or without 0x prefix)
///
/// # Returns
///
/// Parsed facet addresses or error message.
///
/// # Example
///
/// ```rust,ignore
/// let facets = parse_diamond_cut_data("0x00000...")?;
/// if let Some(admin) = facets.admin_facet {
///     println!("Admin facet: {}", admin);
/// }
/// ```
pub fn parse_diamond_cut_data(hex_data: &str) -> Result<DiamondFacets, String> {
    // Remove 0x prefix if present
    let hex_clean = hex_data.strip_prefix("0x").unwrap_or(hex_data);

    // Decode hex to bytes
    let bytes = hex::decode(hex_clean).map_err(|e| format!("Invalid hex: {}", e))?;

    let bytes = Bytes::from(bytes);

    // ABI decode the diamond cut data
    let decoded =
        DiamondCutData::abi_decode(&bytes).map_err(|e| format!("ABI decode failed: {}", e))?;

    let mut facets = DiamondFacets {
        diamond_init: if decoded.initAddress != Address::ZERO {
            Some(decoded.initAddress)
        } else {
            None
        },
        ..Default::default()
    };

    // Extract facet addresses by index
    // The order is deterministic from zkstack deployment:
    // 0: AdminFacet, 1: GettersFacet, 2: MailboxFacet, 3: ExecutorFacet
    for (i, cut) in decoded.facetCuts.iter().enumerate() {
        if cut.facetAddress == Address::ZERO {
            continue;
        }
        match i {
            0 => facets.admin_facet = Some(cut.facetAddress),
            1 => facets.getters_facet = Some(cut.facetAddress),
            2 => facets.mailbox_facet = Some(cut.facetAddress),
            3 => facets.executor_facet = Some(cut.facetAddress),
            _ => {} // Additional facets are ignored
        }
    }

    Ok(facets)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diamond_cut_data_extracts_addresses() {
        // This is a real diamond_cut_data from verifiednet
        // Addresses (from analysis):
        // - AdminFacet: 0x0b282c3dccb1ced1b2760271c61cb2019b332c0b
        // - GettersFacet: 0xe09cfc607f7cd0c8f1951068fbbd655d042c768f
        // - MailboxFacet: 0xc48b4c47c969c53f6458b2cbd5e39bba0f7845ac
        // - ExecutorFacet: 0xcece4f5f31418d42c2a9f5abe97720fd3afaaf32
        // - DiamondInit: 0x44ed5092a94d65f6cbd15a68f1095cd6d8f59ac4
        let hex_data = include_str!("../../testdata/diamond_cut_data.hex");

        // Skip test if test data file doesn't exist
        if hex_data.trim().is_empty() {
            return;
        }

        let result = parse_diamond_cut_data(hex_data.trim());
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let facets = result.unwrap();
        assert!(facets.admin_facet.is_some(), "Admin facet should be parsed");
        assert!(
            facets.getters_facet.is_some(),
            "Getters facet should be parsed"
        );
        assert!(
            facets.mailbox_facet.is_some(),
            "Mailbox facet should be parsed"
        );
        assert!(
            facets.executor_facet.is_some(),
            "Executor facet should be parsed"
        );
        assert!(
            facets.diamond_init.is_some(),
            "DiamondInit should be parsed"
        );
    }

    #[test]
    fn test_parse_invalid_hex() {
        let result = parse_diamond_cut_data("not-valid-hex");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid hex"));
    }

    #[test]
    fn test_parse_empty_data() {
        let result = parse_diamond_cut_data("");
        // Empty hex decodes to empty bytes, which fails ABI decode
        assert!(result.is_err());
    }
}
