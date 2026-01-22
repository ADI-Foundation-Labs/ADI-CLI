#!/usr/bin/env python3
"""
Generate Governance calldata from ABI-encoded Call[].

Default Call struct assumed: (address,uint256,bytes)
Override with: --call-abi "address,uint256,bytes,bool" (example)

Requires:
  pip install eth-abi eth-utils
"""

import argparse
from eth_abi import decode, encode
from eth_utils import keccak, to_hex


ZERO32 = b"\x00" * 32


def strip_0x(s: str) -> str:
    return s[2:] if s.startswith("0x") or s.startswith("0X") else s


def hex_to_bytes(h: str) -> bytes:
    h = strip_0x(h).strip()
    if len(h) % 2 != 0:
        h = "0" + h
    return bytes.fromhex(h)


def bytes32_from_hex(h: str) -> bytes:
    b = hex_to_bytes(h)
    if len(b) != 32:
        raise ValueError(f"Expected 32 bytes (bytes32), got {len(b)} bytes")
    return b


def selector(sig: str) -> bytes:
    return keccak(text=sig)[:4]


def main() -> None:
    p = argparse.ArgumentParser(description="Generate scheduleTransparent/execute calldata from ABI-encoded Call[]")
    p.add_argument(
        "--calls-encoded",
        required=True,
        help="Hex string of ABI-encoded Call[] (i.e., bytes from abi.encode(calls))",
    )
    p.add_argument(
        "--call-abi",
        default="address,uint256,bytes",
        help='Comma-separated Call tuple fields, e.g. "address,uint256,bytes" (default) or "address,uint256,bytes,bool"',
    )
    p.add_argument(
        "--predecessor",
        default="0x" + ("00" * 32),
        help="bytes32 predecessor (default 0x00..00)",
    )
    p.add_argument(
        "--salt",
        default="0x" + ("00" * 32),
        help="bytes32 salt (default 0x00..00)",
    )
    p.add_argument(
        "--delay",
        default=0,
        type=int,
        help="uint256 delay for scheduleTransparent (default 0)",
    )
    args = p.parse_args()

    calls_bytes = hex_to_bytes(args.calls_encoded)
    predecessor = bytes32_from_hex(args.predecessor)
    salt = bytes32_from_hex(args.salt)

    call_tuple_type = f"({args.call_abi})"
    calls_type = f"{call_tuple_type}[]"
    operation_type = f"({calls_type},bytes32,bytes32)"

    # Decode ABI-encoded Call[] into Python values (list of tuples)
    try:
        (calls,) = decode([calls_type], calls_bytes)
    except Exception as e:
        raise SystemExit(
            f"Failed to decode Call[] with type {calls_type}.\n"
            f"Either the input isn't abi.encode(calls), or Call ABI differs.\n"
            f"Error: {e}"
        )

    operation_value = (calls, predecessor, salt)

    # --- scheduleTransparent(Operation,uint256) ---
    sched_sig = f"scheduleTransparent({operation_type},uint256)"
    sched_sel = selector(sched_sig)
    sched_args = encode([operation_type, "uint256"], [operation_value, args.delay])
    schedule_calldata = sched_sel + sched_args

    # --- execute(Operation) ---
    exec_sig = f"execute({operation_type})"
    exec_sel = selector(exec_sig)
    exec_args = encode([operation_type], [operation_value])
    execute_calldata = exec_sel + exec_args

    print("Call tuple type:      ", call_tuple_type)
    print("Calls ABI type:       ", calls_type)
    print("Operation ABI type:   ", operation_type)
    print()
    print("scheduleTransparent signature:", sched_sig)
    print("scheduleTransparent selector: ", to_hex(sched_sel))
    print("scheduleTransparent calldata: ", to_hex(schedule_calldata))
    print()
    print("execute signature:            ", exec_sig)
    print("execute selector:             ", to_hex(exec_sel))
    print("execute calldata:             ", to_hex(execute_calldata))


if __name__ == "__main__":
    main()
