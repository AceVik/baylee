#!/usr/bin/env python3
"""Write the landing page's Impressum blob from the owner's private data.

    python3 scripts/site/impressum_blob.py <private.json> [<out.bin>]

The Impressum page (web/site/impressum.html) names the person behind the
site, but the repository is public and the page is crawled, so no address,
e-mail address or telephone number may appear in it, in its script or in any
file a crawler reads as text. The page's script (web/site/impressum.js)
fetches this blob from a directory only the server has, decodes it and
writes the text into the page.

This is obfuscation, not security: anybody who runs the script, or reads it,
can read the data, which is the point of an Impressum. What it keeps away is
the harvester that greps HTML, JavaScript or a fetched file for something
shaped like an address, an e-mail address or a telephone number. Its key is
split: one half is SCRIPT_KEY (the same constant in impressum.js, a test
holds the two equal), the other a random nonce in each blob.

Blob (ASCII, base64url without padding, so it contains no "@", no "." and no
"+"):

    b64url( MAGIC (4) | VERSION (1) | nonce (16) | ciphertext )

    ciphertext = UTF-8 JSON  XOR  keystream
    keystream  = SHA-256(SCRIPT_KEY | nonce | counter as 4 bytes big-endian)
                 for counter = 0, 1, 2, ...

The JSON is {"v": 1, "fields": {<FIELDS>}}. The input may carry more keys
(an "updated" stamp, for instance); only FIELDS go into the blob.

The output defaults to a file outside the repository and an output path
inside it is refused: the repository must never hold a blob (a test walks
web/ for one). This script never prints a field's value, only sizes.

Standard library only.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import secrets
import sys
from pathlib import Path

MAGIC = b"BYIM"
VERSION = 1
NONCE_LEN = 16
# The script's half of the key. impressum.js carries the same 32 hex digits
# (`SCRIPT_KEY`); change both together.
SCRIPT_KEY = bytes.fromhex("6b1f0c9e52d84a37b0e1f6a2c9d35e48")
FIELDS = (
    "name",
    "street",
    "postcode",
    "city",
    "country_de",
    "country_en",
    "email",
    "phone",
)

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUT = Path.home() / ".local" / "share" / "baylee" / "impressum.bin"


def _keystream(nonce: bytes, length: int) -> bytes:
    out = bytearray()
    counter = 0
    while len(out) < length:
        out += hashlib.sha256(SCRIPT_KEY + nonce + counter.to_bytes(4, "big")).digest()
        counter += 1
    return bytes(out[:length])


def _xor(data: bytes, nonce: bytes) -> bytes:
    return bytes(a ^ b for a, b in zip(data, _keystream(nonce, len(data))))


def encode(fields: dict, nonce: bytes | None = None) -> bytes:
    """The blob for `fields` (every key in FIELDS, each a non-empty string)."""
    missing = [k for k in FIELDS if not isinstance(fields.get(k), str) or not fields[k].strip()]
    if missing:
        raise ValueError("missing or empty field(s): " + ", ".join(missing))
    if nonce is None:
        nonce = secrets.token_bytes(NONCE_LEN)
    if len(nonce) != NONCE_LEN:
        raise ValueError(f"nonce must be {NONCE_LEN} bytes")
    doc = {"v": VERSION, "fields": {k: fields[k].strip() for k in FIELDS}}
    plain = json.dumps(doc, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    raw = MAGIC + bytes([VERSION]) + nonce + _xor(plain, nonce)
    return base64.urlsafe_b64encode(raw).rstrip(b"=")


def decode(blob: bytes) -> dict:
    """The fields back out of a blob (what impressum.js does, for the tests)."""
    text = blob.strip()
    raw = base64.urlsafe_b64decode(text + b"=" * (-len(text) % 4))
    head = len(MAGIC) + 1 + NONCE_LEN
    if len(raw) <= head or raw[: len(MAGIC)] != MAGIC or raw[len(MAGIC)] != VERSION:
        raise ValueError("not an Impressum blob of this version")
    nonce = raw[len(MAGIC) + 1 : head]
    doc = json.loads(_xor(raw[head:], nonce).decode("utf-8"))
    if doc.get("v") != VERSION or not isinstance(doc.get("fields"), dict):
        raise ValueError("unexpected document in the blob")
    return doc["fields"]


def _inside_repo(path: Path) -> bool:
    try:
        path.resolve().relative_to(REPO_ROOT)
    except ValueError:
        return False
    return True


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("json", type=Path, help="the private Impressum data (JSON)")
    ap.add_argument(
        "out",
        type=Path,
        nargs="?",
        default=DEFAULT_OUT,
        help=f"where the blob goes, outside the repository (default {DEFAULT_OUT})",
    )
    args = ap.parse_args(argv)

    if _inside_repo(args.out):
        print(f"refused: {args.out} is inside the repository ({REPO_ROOT})", file=sys.stderr)
        return 2
    try:
        fields = json.loads(args.json.read_text(encoding="utf-8"))
        if not isinstance(fields, dict):
            raise ValueError("the JSON is not an object")
        blob = encode(fields)
    except (OSError, ValueError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 1

    args.out.parent.mkdir(parents=True, exist_ok=True)
    tmp = args.out.with_name(args.out.name + ".tmp")
    tmp.write_bytes(blob + b"\n")
    os.chmod(tmp, 0o644)
    tmp.replace(args.out)
    print(f"wrote {args.out} ({len(blob) + 1} bytes, {len(FIELDS)} fields)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
