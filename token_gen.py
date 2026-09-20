#!/usr/bin/env python3
"""
Generate XOR-obfuscated configuration for opss-terconnect.

Usage:
    python3 token_gen.py <bot_token> <channel_id> <admin_id>

Outputs Python lines to embed in VALORANT.py.
The real token never appears in the source — only XOR-obfuscated bytes.
"""

import sys


def xor_bytes(data: str, key: int) -> list:
    return [b ^ key for b in data.encode("utf-8")]


def main():
    if len(sys.argv) < 4:
        print(f"Usage: {sys.argv[0]} <bot_token> <channel_id> <admin_id>",
              file=sys.stderr)
        sys.exit(1)

    token = sys.argv[1]
    channel_id = sys.argv[2]
    admin_id = sys.argv[3]
    key = 0x42  # XOR key — change for each build if desired

    t = xor_bytes(token, key)
    c = xor_bytes(channel_id, key)
    a = xor_bytes(admin_id, key)

    print("# --- XOR-obfuscated config — paste into VALORANT.py ---")
    print(f"_XOR_KEY = 0x{key:02x}")
    print()
    print("_TOKEN_XORED = [")
    for i, b in enumerate(t):
        comma = "," if i < len(t) - 1 else ""
        print(f"    0x{b:02x}{comma}")
    print("]")
    print()
    print("_CHANNEL_XORED = [")
    for i, b in enumerate(c):
        comma = "," if i < len(c) - 1 else ""
        print(f"    0x{b:02x}{comma}")
    print("]")
    print()
    print("_ADMIN_XORED = [")
    for i, b in enumerate(a):
        comma = "," if i < len(a) - 1 else ""
        print(f"    0x{b:02x}{comma}")
    print("]")
    print("# --- end ---")


if __name__ == "__main__":
    main()
