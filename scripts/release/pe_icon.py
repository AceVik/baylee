#!/usr/bin/env python3
"""Checks that Windows executables carry Baylee's icon resource.

    python3 scripts/release/pe_icon.py <file.exe>...

Reads each file's PE resource table (standard library only, so it runs on
any CI runner and on a developer's Mac) and fails unless it holds icon
group 1 (RT_GROUP_ICON, the ordinal `window_icon.rs` loads and Explorer
shows) with an entry of every size in SIZES, each pointing at an RT_ICON
the file really contains. An executable without one shows Windows' generic
icon in Explorer, the title bar and the taskbar, which is what
0.1.0-beta.3 did (owner, 08.10.2026).
"""

import struct
import sys

RT_ICON = 3
RT_GROUP_ICON = 14
GROUP = 1
# scripts/installers/make-art.py's WINDOWS_SIZES.
SIZES = [16, 24, 32, 48, 64, 128, 256]


class NotPe(Exception):
    pass


def sections(data):
    """The resource directory's RVA and the section table, as (rva, size, raw)."""
    if data[:2] != b"MZ":
        raise NotPe("no MZ header")
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe : pe + 4] != b"PE\0\0":
        raise NotPe("no PE signature")
    count, optional_size = struct.unpack_from("<H12xH", data, pe + 6)
    optional = pe + 24
    magic = struct.unpack_from("<H", data, optional)[0]
    directories = {0x10B: optional + 96, 0x20B: optional + 112}.get(magic)
    if directories is None:
        raise NotPe(f"unknown optional header {magic:#x}")
    resource_rva, _ = struct.unpack_from("<II", data, directories + 2 * 8)
    table = optional + optional_size
    found = []
    for i in range(count):
        at = table + 40 * i
        virtual_size, rva, raw_size, raw = struct.unpack_from("<IIII", data, at + 8)
        found.append((rva, max(virtual_size, raw_size), raw))
    return resource_rva, found


def offset(rva, table):
    for start, size, raw in table:
        if start <= rva < start + size:
            return raw + rva - start
    raise NotPe(f"RVA {rva:#x} is in no section")


def entries(data, base, at):
    """A resource directory's (id, offset, is_directory); named entries skipped."""
    named, ids = struct.unpack_from("<HH", data, at + 12)
    out = []
    for i in range(named + ids):
        name, target = struct.unpack_from("<II", data, at + 16 + 8 * i)
        if name & 0x8000_0000:
            continue
        out.append((name, base + (target & 0x7FFF_FFFF), bool(target & 0x8000_0000)))
    return out


def resources(data):
    """{type: {id: bytes}} for every resource with a numeric type and id."""
    resource_rva, table = sections(data)
    if resource_rva == 0:
        return {}
    base = offset(resource_rva, table)
    out = {}
    for kind, kind_at, _ in entries(data, base, base):
        for ident, ident_at, is_dir in entries(data, base, kind_at):
            leaf = ident_at
            if is_dir:  # the language level; the first language is enough
                langs = entries(data, base, ident_at)
                if not langs:
                    continue
                leaf = langs[0][1]
            rva, size = struct.unpack_from("<II", data, leaf)
            start = offset(rva, table)
            out.setdefault(kind, {})[ident] = data[start : start + size]
    return out


def check(path):
    """None if `path` carries the icon, else what is wrong."""
    with open(path, "rb") as f:
        data = f.read()
    try:
        found = resources(data)
    except (NotPe, struct.error) as err:
        return f"not a readable PE file ({err})"
    groups = found.get(RT_GROUP_ICON, {})
    icons = found.get(RT_ICON, {})
    if GROUP not in groups:
        return f"no icon group {GROUP} (RT_GROUP_ICON ids: {sorted(groups) or 'none'})"
    group = groups[GROUP]
    _, kind, count = struct.unpack_from("<HHH", group, 0)
    if kind != 1:
        return f"icon group {GROUP} is not an icon directory"
    sizes = []
    for i in range(count):
        width, height, _, _, _, _, _, ident = struct.unpack_from("<BBBBHHIH", group, 6 + 14 * i)
        width, height = width or 256, height or 256
        if ident not in icons:
            return f"icon group {GROUP} names RT_ICON {ident}, which is missing"
        if width != height:
            return f"icon {ident} is {width}x{height}, not square"
        sizes.append(width)
    missing = [n for n in SIZES if n not in sizes]
    if missing:
        return f"icon group {GROUP} has {sorted(sizes)}, lacks {missing}"
    return None


def main(paths):
    if not paths:
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        return 2
    failed = False
    for path in paths:
        problem = check(path)
        if problem:
            print(f"::error::{path}: {problem}", file=sys.stderr)
            failed = True
        else:
            print(f"{path}: icon group {GROUP} with {', '.join(map(str, SIZES))} px")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
