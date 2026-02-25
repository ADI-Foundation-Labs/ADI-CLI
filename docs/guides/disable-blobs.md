# l3 da validator set

I’ve set ROLLUP_L1_DA_VALIDATOR from v30-ecosystem.yaml with number 3
NOTE: number 2 also could work
also in server add -

```yaml
l1_sender_pubdata_mode: "Calldata"
```

1. ubuntu@zk-gpu-node-3:~/12_01/l3/zksync-era-v30$ nano v30-ecosystem.yaml

ubuntu@zk-gpu-node-3:~/12_01/l3/zksync-era-v30$ ROLLUP_L1_DA_VALIDATOR=0x58f075ab4e75A610c0753e64094B76e318914322

ubuntu@zk-gpu-node-3:~/12_01/l3$ CALLDATA=$(cast calldata "setDAValidatorPair(address,uint8)" $ROLLUP_L1_DA_VALIDATOR 3)
ubuntu@zk-gpu-node-3:~/12_01/l3$ cast send $CHAIN_ADMIN "multicall((address,uint256,bytes)[],bool)" \
"[($DIAMOND,0,$CALLDATA)]" true \
--private-key $CHAIN_GOV_PK \
--rpc-url $RPC

blockHash               0x4827d50c38c0fa36966ba8ccf49cada2695754bce2869b687b0fa4d5195ff38e
blockNumber             9150
contractAddress
cumulativeGasUsed       56506
effectiveGasPrice       552000000001
from                    0xd8a1b668c32300011Ba62C3E50D71a42dc339Cd6
gasUsed                 56506
logs                    [{"address":"0xd90ebb9189752153b68723187616174e9e466525","topics":["0x08b0b676d456a0431162145d2821f30c665b69ca626521c937ba7a77a29ed67c","0x00000000000000000000000058f075ab4e75a610c0753e64094b76e318914322","0x00000000000000000000000058f075ab4e75a610c0753e64094b76e318914322"],"data":"0x","blockHash":"0x4827d50c38c0fa36966ba8ccf49cada2695754bce2869b687b0fa4d5195ff38e","blockNumber":"0x23be","blockTimestamp":"0x696f9706","transactionHash":"0x023f9ebf47b1d2c9c4732081969c771b7457e4b152e0038cce79f34c8692cd94","transactionIndex":"0x0","logIndex":"0x0","removed":false},{"address":"0xd90ebb9189752153b68723187616174e9e466525","topics":["0xce805def6fd033d429928feccdb0bcbff328d8e318308bd4cd4ff893999f9d76","0x0000000000000000000000000000000000000000000000000000000000000002","0x0000000000000000000000000000000000000000000000000000000000000003"],"data":"0x","blockHash":"0x4827d50c38c0fa36966ba8ccf49cada2695754bce2869b687b0fa4d5195ff38e","blockNumber":"0x23be","blockTimestamp":"0x696f9706","transactionHash":"0x023f9ebf47b1d2c9c4732081969c771b7457e4b152e0038cce79f34c8692cd94","transactionIndex":"0x0","logIndex":"0x1","removed":false},{"address":"0xef19746f9170d997e7a5a0456454d5595f34932d","topics":["0x157c677a40c50f832f816d7b59c8c3e94774acae328c8ccb145b73aea7566d75"],"data":"0x000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000140000000000000000000000000d90ebb9189752153b68723187616174e9e4665250000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000442765d07900000000000000000000000058f075ab4e75a610c0753e64094b76e3189143220000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","blockHash":"0x4827d50c38c0fa36966ba8ccf49cada2695754bce2869b687b0fa4d5195ff38e","blockNumber":"0x23be","blockTimestamp":"0x696f9706","transactionHash":"0x023f9ebf47b1d2c9c4732081969c771b7457e4b152e0038cce79f34c8692cd94","transactionIndex":"0x0","logIndex":"0x2","removed":false}]
logsBloom               0x04000000000000008000000000000000000000000080000000000000000000000000000000000000000000000000000000010020020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200000000200000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000804000000100002000000000001000880400000000000000400000000100000000400000000000020000000000000000000000000000000000000000000000000100000000000000002000000000008000000000000000000000
root
status                  1 (success)
transactionHash         0x023f9ebf47b1d2c9c4732081969c771b7457e4b152e0038cce79f34c8692cd94
transactionIndex        0
type                    2
blobGasPrice
blobGasUsed
authorizationList
to                      0xEF19746f9170d997e7A5a0456454D5595f34932D
l2ToL1Logs             []

## For ZKsync OS without blobs:

If you want to use **ZKsync OS server v0.10.1** without blobs, you have two options:

### Option 1: Validium (no DA on L1)

`# Use ValidiumL1DAValidator with EMPTY_NO_DA (1)
L1_DA_VALIDATOR="0x7553e9C5374eEc7E8B16A98cd2EcBff189ee2609"  # ValidiumL1DAValidator
CALLDATA=$(cast calldata "setDAValidatorPair(address,uint8)" $L1_DA_VALIDATOR 1)`

### Option 2: Calldata DA (pubdata via calldata, not blobs)

`# Use RollupL1DAValidator with PUBDATA_KECCAK256 (2)
L1_DA_VALIDATOR="0x76201Eb44390e3cb311Aee1DCCdB943d0766ec34"  # RollupL1DAValidator
CALLDATA=$(cast calldata "setDAValidatorPair(address,uint8)" $L1_DA_VALIDATOR 2)`

## Default in contracts v0.29.x (Era, not ZKsync OS):

In **v0.29.x** (original Era contracts), the default was:

- **`BLOBS_AND_PUBDATA_KECCAK256` (3)** with `RollupL1DAValidator`

## Summary Table

| Scheme                      | Value | L1 DA Validator            | Use Case                         |
| --------------------------- | ----- | -------------------------- | -------------------------------- |
| EMPTY_NO_DA                 | 1     | ValidiumL1DAValidator      | Validium (no L1 DA)              |
| PUBDATA_KECCAK256           | 2     | RollupL1DAValidator        | Calldata-based rollup (no blobs) |
| BLOBS_AND_PUBDATA_KECCAK256 | 3     | RollupL1DAValidator        | Era with blobs (default v0.29)   |
| BLOBS_ZKSYNC_OS             | 4     | BlobsL1DAValidatorZKsyncOS | ZKsync OS with blobs             |

## Your command for non-blobs ZKsync OS:

`# For calldata-based pubdata (no blobs, but still rollup)
L1_DA_VALIDATOR="0x76201Eb44390e3cb311Aee1DCCdB943d0766ec34"
CALLDATA=$(cast calldata "setDAValidatorPair(address,uint8)" $L1_DA_VALIDATOR 2)

cast send $CHAIN_ADMIN "multicall((address,uint256,bytes)[],bool)" \
  "[($DIAMOND,0,$CALLDATA)]" true \
  --private-key $CHAIN_GOVERNOR_PK \
  --rpc-url $RPC`
