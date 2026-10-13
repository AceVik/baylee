#!/usr/bin/env python3
"""Write the landing page's Impressum blob from the owner's private data.

    python3 scripts/site/impressum_blob.py <private.json> [<out.bin>]

The Impressum page (web/site/impressum.html) names the person behind the
site, but the repository is public and the page is crawled, so no address,
e-mail address or telephone number may appear in it, in its script or in any
file a crawler reads as text. The page's script (web/site/impressum.js)
fetches this blob from a directory only the server has, decodes it and
places each character of each field into the page on its own, in a shuffled
order among decoys, so that the values are never one string in the blob,
in the decoded document or in the page's DOM (impressum.js says how).

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

The JSON is {"v": 2, "fields": {<FIELD>: [[position, code point], ...]}}:
every field a list of its characters as numbers, each with its position,
the list shuffled, so not even the decoded document holds a value as text.
A field is at most MAX_LEN characters (the page's CSS orders that many).
The input may carry more keys (an "updated" stamp, for instance); only
FIELDS go into the blob.

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
VERSION = 2
NONCE_LEN = 16
# web/site/site.css has an order class for each position, .o0 to .o63
MAX_LEN = 64
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


def _needles(fields: dict) -> set[str]:
    """What must not be readable in a blob: every value, every word of three
    letters or more, the telephone number's digits, both halves of the
    e-mail address."""
    out: set[str] = set()
    for value in fields.values():
        out.add(value)
        out.update(w for w in value.split() if len(w) >= 3)
    out.add("".join(c for c in fields["phone"] if c.isdigit()))
    local, _, domain = fields["email"].partition("@")
    out.update((local, domain))
    return {n for n in out if n}


def _readable(blob: bytes, fields: dict) -> bool:
    """Whether any needle shows in the blob as text, in the raw bytes behind
    the base64, or in their hex. A random nonce makes that unlikely, not
    impossible (a three-letter word in 400 base64 characters), so encode()
    checks every blob it writes."""
    text = blob.decode("ascii").lower()
    raw = base64.urlsafe_b64decode(blob + b"=" * (-len(blob) % 4))
    hexed = raw.hex()
    for n in _needles(fields):
        b = n.encode("utf-8")
        if n.lower() in text or b in raw or b.hex() in hexed:
            return True
    return False


def encode(fields: dict, nonce: bytes | None = None) -> bytes:
    """The blob for `fields` (every key in FIELDS, each a non-empty string).
    A blob in which a value or a word of it could be read is never returned:
    the characters are shuffled again (and a new nonce drawn unless one is
    given), and after 64 tries it raises."""
    missing = [k for k in FIELDS if not isinstance(fields.get(k), str) or not fields[k].strip()]
    if missing:
        raise ValueError("missing or empty field(s): " + ", ".join(missing))
    if nonce is not None and len(nonce) != NONCE_LEN:
        raise ValueError(f"nonce must be {NONCE_LEN} bytes")
    clean = {k: fields[k].strip() for k in FIELDS}
    long = [k for k in FIELDS if len(clean[k]) > MAX_LEN]
    if long:
        raise ValueError(f"field(s) longer than {MAX_LEN} characters: " + ", ".join(long))
    rng = secrets.SystemRandom()
    for _ in range(64):
        scattered = {}
        for k in FIELDS:
            pairs = [[i, ord(c)] for i, c in enumerate(clean[k])]
            rng.shuffle(pairs)
            scattered[k] = pairs
        doc = {"v": VERSION, "fields": scattered}
        plain = json.dumps(doc, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
        # a given nonce stays; the shuffle alone changes the blob
        n = nonce if nonce is not None else secrets.token_bytes(NONCE_LEN)
        raw = MAGIC + bytes([VERSION]) + n + _xor(plain, n)
        blob = base64.urlsafe_b64encode(raw).rstrip(b"=")
        if not _readable(blob, clean):
            return blob
    raise ValueError("every blob tried showed part of the data; try again")


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
    out = {}
    for k, pairs in doc["fields"].items():
        chars = [""] * len(pairs)
        for i, cp in pairs:
            chars[i] = chr(cp)
        out[k] = "".join(chars)
    return out


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
