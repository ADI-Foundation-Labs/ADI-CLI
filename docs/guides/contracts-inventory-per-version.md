# Contracts inventory per version

Created by: Mykhailo Slyvka
Edited at: January 27, 2026 2:42 PM
Status: Done

# Clean ecosystem smart contract deployment

### v29.11 deployment deploys 57 contracts:

zkstack CLI during ecosystem init run several forge scripts:

1. **Forge Library Deployments -** Before any script execution, Forge automatically deploys libraries with `public` or `external` functions as separate contracts via the Create2 Factory:
    - BytecodeUtils
    - Utils
2. **L1 Core Ecosystem Contracts:**
    - Governance
    - ChainAdminOwnable
    - ProxyAdmin (via Create2AndTransfer)
    - L1Bridgehub (Implementation + Proxy)
    - L1MessageRoot (Implementation + Proxy)
    - L1Nullifier (Implementation + Proxy)
    - L1AssetRouter (Implementation + Proxy)
    - BridgedStandardERC20 (via Create2AndTransfer)
    - BridgedTokenBeacon
    - L1NativeTokenVault (Implementation + Proxy)
    - L1ERC20Bridge (Implementation + Proxy)
    - CTMDeploymentTracker (Implementation + Proxy)
    - L1ChainAssetHandler (Implementation + Proxy)
3. **Deploy CTM:**
    - **RollupDAManager (via Create2AndTransfer)**
    - **RollupL1DAValidator**
    - **ValidiumL1DAValidator**
    - **DummyAvailBridge**
    - **DummyVectorX**
    - **AvailL1DAValidator**
    - **BytecodesSupplier**
    - **VerifierFflonk**
    - **VerifierPlonk**
    - **DualVerifier**
    - **DefaultUpgrade**
    - **L1GenesisUpgrade**
    - **ValidatorTimelock** (Implementation + Proxy) ???
    - ProxyAdmin for ServerNotifier **(via Create2AndTransfer)**
    - **ServerNotifier** (Implementation + Proxy)
    - **ExecutorFacet**
    - **AdminFacet**
    - **MailboxFacet**
    - **GettersFacet**
    - **DiamondInit**
    - **ChainTypeManager** (Implementation + Proxy)
4. Deploy Chain:
    - Governance
    - ChainAdminOwnable
    - **ProxyAdmin (via Create2AndTransfer)**
    - DiamondProxy
5. **Multicall3 (Optional for chains where it is not deployed)**

So in total deployment of contracts v0.29.11 deploys **57 contracts**. 1 contract (Multicall3) is optional - it will not be deployed if it is already created on chain.

### v30.2 deployment deploy 56 contracts:

The diffrence from v29 version is:

1. v29 L1VerifierFflonk replaced by `ZKsyncOSVerifierFflonk` 
2. v29 L1VerifierPlonk replaced by `ZKsyncOSVerifierPlonk` 
3. v29 DualVerifier replaced by `ZKsyncOSDualVerifier`
4. v29 ChainTypeManager replaced by `ZKsyncOSChainTypeManager` 
5. Utils libraries (`BytecodeUtils` and `Utils` ) are not deployed because they were changed to `internal` 
6. NEW contract `BlobsL1DAValidatorZKsyncOS` was added in v30.

# Upgrade smart contract deployment

### v30.0 upgrade deploys 30 new contracts:

1. Deploy **ZKsyncOSVerifierFflonk**
2. Deploy **ZKsyncOSVerifierPlonk**
3. Deploy **ZKsyncOSDualVerifier**
4. Deploy **UpgradeStageValidator**
5. Deploy **RollupDAManager**
6. Deploy **RollupL1DAValidator**
7. Deploy **BlobsL1DAValidatorZKsyncOS**
8. **Deploy ValidiumL1DAValidator**
9. Deploy **DummyAvailBridge**
10. Deploy **AvailL1DAValidator**
11. Deploy **L1ZKsyncOSV30Upgrade**
12. Deploy **L1GenesisUpgrade**
13. Deploy **L1Bridgehub** (impl)
14. Deploy **L1Nullifier** (impl)
15. Deploy  **L1AssetRouter** (impl)
16. Deploy  **L1NativeTokenVault** (impl)
17. Deploy  **L1ERC20Bridge** (impl)
18. Deploy  **BridgedStandardERC20** (impl)
19. Deploy  **GovernanceUpgradeTimer**
20. Deploy **L1MessageRoot** (impl)
21. Deploy **CTMDeploymentTracker** (impl)
22. Deploy **ExecutorFacet**
23. Deploy **AdminFacet**
24. Deploy **MailboxFacet**
25. Deploy **GettersFacet**
26. Deploy **DiamondInit**
27. Deploy **ZKsyncOSChainTypeManager** (impl)
28. Deploy **ServerNotifier** (impl)
29. **Deploy ValidatorTimelock** (impl)
30. Deploy **DiamondProxy (for forge verification only lol -** https://github.com/matter-labs/era-contracts/blob/main/l1-contracts/deploy-scripts/upgrade/DefaultEcosystemUpgrade.s.sol#L230**)**

**Also run 6 function calls:**

1. addVerifier(5, fflonk, plonk) → DualVerifier
2. transferOwnership(governance) → DualVerifier
3. acceptOwnership() → RollupDAManager
4. updateDAPair(rollupValidator, ROLLUP, true) → RollupDAManager
5. updateDAPair(blobsValidator, BLOBS_ZKSYNC_OS, true) → RollupDAManager
6. transferOwnership(governance) → RollupDAManager

### v30.2 upgrade deploys 4 new contracts:

1. Deploy ZKsyncOSVerifierFflonk
2. Deploy ZKsyncOSVerifierPlonk
3. Deploy ZKsyncOSDualVerifier
4. Deploy DefaultUpgrade

Also run 2 calls:

1. addVerifier(6, fflonk, plonk) → DualVerifier
2. transferOwnership(owner) → DualVerifier

So totally 6 txs