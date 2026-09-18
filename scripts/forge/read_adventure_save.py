#!/usr/bin/env python3
"""Read the decks out of a Forge Adventure save.

A Forge Adventure save is a zlib stream whose object graph carries a
`SaveFileData extends HashMap<String, byte[]>`: every value is itself an
independent Java serialization stream. The decks live under the keys
`deck_name_<n>`, `deck_<n>`, `sideBoardCards_<n>` and `commanderCards_<n>`,
and each card is one string written by `CardDb.CardRequest.compose`:

    <count> <name>[+]|<SET>|[<collector number>][|${flag=value,...}]

`+` is `CardDb.foilSuffix`, the brackets are what `preprocessCollectorNumber`
puts there, and the fourth segment is `getFlagSegment`. Nothing in it says a
language: Forge keeps that as one global preference
(`UI_CARD_DOWNLOAD_LANG`), so a row here is a printing and not a translation.

Nothing is copied out of Forge but the owner's own save. The edition files
this script's sibling reads are Forge's and stay where they are.

Usage:
    read_adventure_save.py <save.sav> [--json]

Without `--json` it prints one human-readable block per non-empty deck.
"""

from __future__ import annotations

import io
import json
import re
import struct
import sys
import zlib

# What a composed card request looks like. Anchored on both ends, because a
# row this does not match is a row we would otherwise import as a card name
# with punctuation in it.
ROW = re.compile(r"^(\d+) (.+?)(\+?)\|([^|]*)\|\[([^\]]*)\](?:\|\$\{(.*)\})?$")

# Java serialization, the four tags this reader needs.
TC_STRING, TC_REFERENCE, TC_NULL, TC_ARRAY = 0x74, 0x71, 0x70, 0x75
TC_CLASSDESC, TC_ENDBLOCKDATA, TC_LONGSTRING = 0x72, 0x78, 0x7C
BASE_HANDLE = 0x7E0000


class Unreadable(Exception):
    """The save did not have the shape this reader knows."""


class Stream:
    """Just enough of a Java object stream to read a `String[]`.

    Handles are the part a naive scanner gets wrong: Java writes a repeated
    `String` *object* as a four-byte back-reference rather than as its
    characters, so a reader that only matches `t\\x00<len>` silently drops
    every row whose exact spelling already appeared. Both decks here have
    duplicates -- ten copies of one basic land are one object -- so this is
    not a hypothetical.
    """

    def __init__(self, data: bytes) -> None:
        self.data = data
        self.pos = 0
        self.handles: list[object] = []

    def u8(self) -> int:
        v = self.data[self.pos]
        self.pos += 1
        return v

    def u16(self) -> int:
        v = struct.unpack_from(">H", self.data, self.pos)[0]
        self.pos += 2
        return v

    def u32(self) -> int:
        v = struct.unpack_from(">I", self.data, self.pos)[0]
        self.pos += 4
        return v

    def utf(self, long: bool = False) -> str:
        n = self.u32() if long else self.u16()
        s = self.data[self.pos : self.pos + n].decode("utf-8")
        self.pos += n
        return s

    def new_handle(self, obj: object) -> object:
        self.handles.append(obj)
        return obj

    def reference(self) -> object:
        i = self.u32() - BASE_HANDLE
        if not 0 <= i < len(self.handles):
            raise Unreadable(f"back-reference to handle {i}, of {len(self.handles)}")
        return self.handles[i]

    def element(self) -> str | None:
        """One array element: a string, a back-reference to one, or null."""
        tag = self.u8()
        if tag == TC_NULL:
            return None
        if tag == TC_STRING:
            return self.new_handle(self.utf())  # type: ignore[return-value]
        if tag == TC_LONGSTRING:
            return self.new_handle(self.utf(long=True))  # type: ignore[return-value]
        if tag == TC_REFERENCE:
            v = self.reference()
            if not isinstance(v, str):
                raise Unreadable("a back-reference that is not a string")
            return v
        raise Unreadable(f"unexpected element tag 0x{tag:02x} at {self.pos - 1}")


def string_array(blob: bytes) -> list[str]:
    """The `String[]` a `SaveFileData` value holds.

    The declared length is read and then *checked* against what came out.
    A reader that stops early on an unexpected byte and reports what it got
    is the failure this repo has seen more than once.
    """
    if blob[:4] != b"\xac\xed\x00\x05":
        raise Unreadable("value is not a serialization stream")
    s = Stream(blob)
    s.pos = 4
    if s.u8() != TC_ARRAY:
        raise Unreadable("value is not an array")
    if s.u8() != TC_CLASSDESC:
        raise Unreadable("array has no class descriptor of its own")
    name = s.utf()
    if name != "[Ljava.lang.String;":
        raise Unreadable(f"array is {name}, not String[]")
    s.pos += 8  # serialVersionUID
    s.pos += 1  # flags
    if s.u16() != 0:
        raise Unreadable("array class declares fields")
    s.new_handle(name)  # the class descriptor takes a handle
    if s.u8() != TC_ENDBLOCKDATA:
        raise Unreadable("class descriptor is not terminated")
    if s.u8() != TC_NULL:
        raise Unreadable("array class has a superclass")
    s.new_handle("<array>")  # the array object takes the next handle
    count = s.u32()
    out = [s.element() for _ in range(count)]
    if len(out) != count:
        raise Unreadable(f"array declares {count} entries, {len(out)} read")
    if s.pos != len(blob):
        raise Unreadable(f"{len(blob) - s.pos} bytes left after {count} entries")
    # A null and an empty string are both "no card here" -- Forge writes a
    # fixed-width slot array and leaves the unused ones blank. Dropped after
    # the count check and never before it, so a short read still fails.
    return [e for e in out if e]


def utf_value(blob: bytes) -> str:
    """The `String` a `SaveFileData` value holds, written with `writeUTF`."""
    if blob[:4] != b"\xac\xed\x00\x05":
        raise Unreadable("value is not a serialization stream")
    if blob[4] != 0x77:  # TC_BLOCKDATA
        raise Unreadable("value is not block data")
    # `\xac\xed\x00\x05` `w` <block length> then `writeUTF`: a two-byte
    # length and the bytes. Reading from the wrong offset does not fail --
    # it prepends the length's own low byte to the name, which looks like a
    # stray control character and reads as data.
    (length,) = struct.unpack_from(">H", blob, 6)
    if len(blob) - 8 != length:
        raise Unreadable(f"string declares {length} bytes, {len(blob) - 8} present")
    return blob[8:].decode("utf-8")


def entries(raw: bytes) -> dict[str, bytes]:
    """Every `<key>` -> `<byte[]>` pair in the decompressed save.

    Scanned rather than deserialized: the save's root is a GDX-flavoured
    object graph whose header carries a `Pixmap`, and the only thing wanted
    here is the flat map inside it. Each pair is written as a `TC_STRING`
    key followed by the `byte[]` value, and every value that matters starts
    with the stream magic -- which is what makes the scan checkable rather
    than hopeful.
    """
    found: dict[str, bytes] = {}
    for m in re.finditer(rb"\x74(..)", raw, re.S):
        n = struct.unpack(">H", m.group(1))[0]
        start = m.end()
        try:
            key = raw[start : start + n].decode("utf-8")
        except UnicodeDecodeError:
            continue
        p = start + n
        # `uq\x00~\x00XX` -- a byte[] whose class is a back-reference -- then
        # a four-byte length. Anything else is not a value of this map.
        if raw[p : p + 2] != b"uq" or raw[p + 2 : p + 3] != b"\x00":
            continue
        p += 6
        (size,) = struct.unpack_from(">I", raw, p)
        p += 4
        if size > len(raw) - p:
            continue
        found.setdefault(key, raw[p : p + size])
    return found


def decks(raw: bytes) -> list[dict]:
    """Every named deck slot, in slot order."""
    data = entries(raw)
    out = []
    for i in range(100):
        key = f"deck_name_{i}"
        if key not in data:
            continue
        name = utf_value(data[key])
        deck = {
            "slot": i,
            "name": name,
            "main": string_array(data[f"deck_{i}"]) if f"deck_{i}" in data else [],
            "sideboard": (
                string_array(data[f"sideBoardCards_{i}"])
                if f"sideBoardCards_{i}" in data
                else []
            ),
            "commanders": (
                string_array(data[f"commanderCards_{i}"])
                if f"commanderCards_{i}" in data
                else []
            ),
        }
        out.append(deck)
    return out


def parse_row(row: str) -> dict:
    """One composed card request, or an error naming the row."""
    m = ROW.match(row)
    if not m:
        raise Unreadable(f"row is not a card request: {row!r}")
    count, name, foil, setc, number, flags = m.groups()
    return {
        "count": int(count),
        "name": name,
        "foil": bool(foil),
        "set": setc,
        "number": number,
        "flags": flags or None,
    }


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    raw = zlib.decompress(io.open(argv[1], "rb").read())
    found = decks(raw)
    if not found:
        print("no deck slot found -- is this a Forge Adventure save?", file=sys.stderr)
        return 1
    if "--json" in argv:
        for d in found:
            for section in ("main", "sideboard", "commanders"):
                d[section] = [parse_row(r) for r in d[section]]
        json.dump(found, sys.stdout, indent=2, ensure_ascii=False)
        print()
        return 0
    for d in found:
        total = sum(parse_row(r)["count"] for r in d["main"])
        if not total and not d["commanders"]:
            continue
        print(
            f"slot {d['slot']:2}  {d['name']!r}: {total} main"
            f" / {sum(parse_row(r)['count'] for r in d['sideboard'])} sideboard"
            f" / {len(d['commanders'])} commander"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
