#!/usr/bin/env python3
"""Write the landing page's card status: every set, every card, how far each is.

    python3 scripts/site/card_status.py [--rev REV] [--out web/site/data]
                                        [--sets-cache PATH] [--bulk PATH]

Reads one git revision of this repository (default `HEAD`), never the working
tree, so what the page says is what that commit holds:

- the CardIndex ledger (`crates/baylee-cards-index/src/generated.rs`): every
  card there is, the set it first appeared in, ordered by first appearance,
  which is release order (the ledger is assigned that way and append-only);
- every card file under `crates/baylee-cards/src/cards/`: its `oracle_id`,
  its `scryfall_id` and its `coverage` (`Implemented`, `Partial("reason")`,
  or none, which is a generated stub and `Unimplemented` by default);
- `data/unplayable.tsv`: the cards never built (ante, dexterity, subgame, un-set),
  which a set is counted complete without.

Status per card, first match wins: `never` (unplayable.tsv), `implemented`,
`partial` (with its reason), `unimplemented` (a stub in the pool), `absent`
(no card file yet). The vocabulary is the feedback console's
(`web/feedback/src/api.ts`).

Set names and dates come from Scryfall's set list, one request to
`https://api.scryfall.com/sets`, kept at `--sets-cache` (gitignored,
under `target/`) and reused; without it the names already in the previous
output are reused, else the page shows the code.

Card pictures: the page builds a `cards.scryfall.io` URL from a printing id
(`docs/legal.md` §3; the browser loads it from Scryfall, nothing is
re-hosted). Only a card in the pool (implemented, partial, unimplemented)
or on the never-built list has one, and never a Universes Beyond printing
(Scryfall's `security_stamp` "triangle", or a set in `UB_SETS`): every other
card is listed by name only, and its entry carries no Scryfall printing id.
A pool card has the id its card file names; `--bulk` names a Scryfall
`default_cards` (or `oracle_cards`) bulk file, which gives the printing from
the set it is listed under where one exists, else any (read line by line,
Scryfall writes one card per line). Without it the ids already in the
previous output are kept.

Where the printing's own Scryfall payload is at hand (the bulk file, or
the per-card cache `--card-cache`, default `data/scryfall-cache`, which
codegen fills), a pictured card also gets `uri`, its page on scryfall.com
(artist and printing; the page links each tile there), and `artist`; its
`security_stamp` decides Universes Beyond.

Standard library only. Writes `<out>/sets.json` (the overview) and
`<out>/sets/set-<code>.json` (one set's cards, fetched when it is opened; the
prefix keeps a code such as `con` from naming a reserved Windows file).
"""

import argparse
import gzip
import io
import json
import os
import re
import subprocess
import sys
import tarfile
import urllib.request
from datetime import datetime, timezone

LEDGER = "crates/baylee-cards-index/src/generated.rs"
CORE_INDEX = "crates/baylee-core/src/generated/index"
CARDS = "crates/baylee-cards/src/cards"
UNPLAYABLE = "data/unplayable.tsv"

ROW = re.compile(
    r'Row \{ index: CardIndex::new\((\d+)\), oracle_id: "([0-9a-f-]{36})", '
    r'constant: "[A-Z0-9_]+", set: "([a-z0-9]+)", name: "((?:[^"\\]|\\.)*)" \}'
)
SET_HEADER = re.compile(r"^// (\d+) cards? first printed in `([a-z0-9]+)`\.", re.M)
UUID = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
FIELD_ORACLE = re.compile(r'\boracle_id\s*=\s*"([0-9a-f-]{36})"')
FIELD_SCRYFALL = re.compile(r'\bscryfall_id\s*=\s*"([0-9a-f-]{36})"')
COVERAGE = re.compile(r"\bcoverage\s*=\s*Coverage::(Implemented|Unimplemented|Partial)\b")
STRING = re.compile(r'"((?:[^"\\]|\\.)*)"', re.S)
STUB_MARKER = "GENERATED STUB"

UA = "baylee-site-card-status/1 (+https://github.com/AceVik/baylee)"

# Statuses that may show a picture: the pool, and the never-built list.
PICTURED = ("implemented", "partial", "unimplemented", "never")

# Universes Beyond sets (crossovers with other rights holders, whose pictures
# are not Wizards' to permit; docs/legal.md §3). The security stamp catches
# them and the crossover drops of other sets (Secret Lair); this list holds
# where a printing's payload is not at hand.
UB_SETS = frozenset(
    "ltr ltc who pip acr fin fic 40k bot rex msh msc spe tla tle ptla clu".split()
)


def pictured(status, listed_set, img, facts):
    """The picture a card may show: `img` (a printing id) when the card is in
    the pool or on the never-built list and the printing is no Universes
    Beyond one, by its stamp, else by its set, else by the set the card is
    listed under; else None."""
    if not img or status not in PICTURED:
        return None
    stamp, _uri, _artist, pset = facts.get(img, (None, None, None, None))
    if stamp == "triangle" or (pset or listed_set) in UB_SETS or listed_set in UB_SETS:
        return None
    return img


def printing_facts(path):
    """{printing id: (security_stamp, scryfall_uri, artist, set)} from a
    directory of Scryfall card payloads, one JSON object per file."""
    facts = {}
    if not path or not os.path.isdir(path):
        return facts
    for fn in os.listdir(path):
        if not fn.endswith(".json"):
            continue
        try:
            with open(os.path.join(path, fn), encoding="utf-8") as f:
                c = json.load(f)
        except (OSError, ValueError):
            continue
        if isinstance(c, dict) and UUID.match(str(c.get("id", ""))):
            facts[c["id"]] = fact(c)
    return facts


def fact(c):
    uri = (c.get("scryfall_uri") or "").split("?")[0]
    artist = c.get("artist") or ((c.get("card_faces") or [{}])[0].get("artist"))
    return (c.get("security_stamp"), uri if uri.startswith("https://scryfall.com/card/") else None, artist, c.get("set"))


def tree(rev, paths):
    """Every file under `paths` at `rev`, as {path: text}, from one `git archive`."""
    raw = subprocess.run(
        ["git", "archive", "--format=tar", rev, "--", *paths],
        check=True,
        capture_output=True,
    ).stdout
    files = {}
    with tarfile.open(fileobj=io.BytesIO(raw)) as tar:
        for m in tar:
            if m.isfile():
                files[m.name] = tar.extractfile(m).read().decode("utf-8")
    return files


def rust_string(body):
    """A Rust string literal's value: `\\` + newline + indentation joins, escapes undone."""
    out, i = [], 0
    while i < len(body):
        c = body[i]
        if c == "\\" and i + 1 < len(body):
            n = body[i + 1]
            if n == "\n":
                i += 2
                while i < len(body) and body[i] in " \t\n\r":
                    i += 1
                continue
            out.append({"n": "\n", "t": "\t", '"': '"', "\\": "\\", "'": "'"}.get(n, n))
            i += 2
            continue
        out.append(c)
        i += 1
    return "".join(out)


def partial_reason(text, start):
    """The reason in `Coverage::Partial(...)`: every string literal up to the closing paren."""
    open_paren = text.index("(", start)
    depth, i, parts = 0, open_paren, []
    while i < len(text):
        c = text[i]
        if c == '"':
            m = STRING.match(text, i)
            parts.append(rust_string(m.group(1)))
            i = m.end()
            continue
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                break
        i += 1
    return " ".join(p.strip() for p in parts) or None


def read_card(path, text):
    oracle = FIELD_ORACLE.search(text)
    if not oracle:
        sys.exit(f"{path}: no oracle_id")
    scry = FIELD_SCRYFALL.search(text)
    covs = COVERAGE.findall(text)
    if len(covs) > 1:
        sys.exit(f"{path}: {len(covs)} coverage fields")
    if not covs:
        # A generated stub writes no coverage: CardDef's default is Unimplemented.
        if STUB_MARKER not in text:
            sys.exit(f"{path}: no coverage and no stub marker; read it by hand")
        status, note = "unimplemented", None
    else:
        kind = covs[0]
        status = {"Implemented": "implemented", "Unimplemented": "unimplemented", "Partial": "partial"}[kind]
        note = partial_reason(text, COVERAGE.search(text).start()) if kind == "Partial" else None
    return oracle.group(1), (scry.group(1) if scry else None), status, note


def scryfall_sets(cache, fetch):
    if not os.path.exists(cache) and fetch:
        os.makedirs(os.path.dirname(cache) or ".", exist_ok=True)
        req = urllib.request.Request(
            "https://api.scryfall.com/sets", headers={"User-Agent": UA, "Accept": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=60) as r:
            data = r.read()
        with open(cache, "wb") as f:
            f.write(data)
    if not os.path.exists(cache):
        return {}
    with open(cache, encoding="utf-8") as f:
        doc = json.load(f)
    return {s["code"]: s for s in doc["data"]}


def bulk_printings(path, wanted, facts=None):
    """{oracle_id: {set: id, '*': id}} from a Scryfall bulk file, read a line at a time;
    each chosen printing's facts (`fact`) go into `facts`."""
    best = {}
    every = {}
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt", encoding="utf-8") as f:
        for line in f:
            line = line.strip().rstrip(",")
            if not line.startswith("{"):
                continue
            c = json.loads(line)
            oid = c.get("oracle_id") or (c.get("card_faces") or [{}])[0].get("oracle_id")
            if oid not in wanted or c.get("lang", "en") != "en":
                continue
            if c.get("image_status") in ("missing", "placeholder"):
                continue
            # A printing's front picture: on the card, or on its first face.
            if "image_uris" not in c and not any("image_uris" in x for x in c.get("card_faces") or []):
                continue
            # never a Universes Beyond printing where another exists
            key = (c.get("security_stamp") == "triangle", c.get("promo", False), c.get("digital", False), num_key(c.get("collector_number", "")))
            if facts is not None:
                every[c["id"]] = fact(c)
            slot = best.setdefault(oid, {})
            for s in (c["set"], "*"):
                if s not in slot or key < slot[s][0]:
                    slot[s] = (key, c["id"])
    chosen = {o: {s: v[1] for s, v in slots.items()} for o, slots in best.items()}
    if facts is not None:
        for slots in chosen.values():
            for pid in slots.values():
                facts[pid] = every[pid]
    return chosen


def fetch_bulk(cache_dir):
    """Download Scryfall's `default_cards` bulk file (JSON lines, gzip) once; its path."""
    req = urllib.request.Request(
        "https://api.scryfall.com/bulk-data/default-cards", headers={"User-Agent": UA, "Accept": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=60) as r:
        meta = json.load(r)
    uri = meta.get("jsonl_download_uri") or meta["download_uri"]
    path = os.path.join(cache_dir, os.path.basename(uri))
    if not os.path.exists(path):
        os.makedirs(cache_dir, exist_ok=True)
        req = urllib.request.Request(uri, headers={"User-Agent": UA})
        with urllib.request.urlopen(req, timeout=600) as r, open(path + ".part", "wb") as f:
            while True:
                chunk = r.read(1 << 20)
                if not chunk:
                    break
                f.write(chunk)
        os.replace(path + ".part", path)
    return path


def num_key(n):
    m = re.match(r"(\d+)(.*)", n)
    return (int(m.group(1)), m.group(2)) if m else (1 << 30, n)


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    repo = os.path.dirname(os.path.dirname(here))
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--rev", default="HEAD", help="the git revision to read (default HEAD)")
    ap.add_argument("--out", default=os.path.join(repo, "web/site/data"))
    ap.add_argument("--sets-cache", default=os.path.join(repo, "target/site-cache/scryfall-sets.json"))
    ap.add_argument("--no-fetch", action="store_true", help="never ask Scryfall for the set list")
    ap.add_argument("--bulk", help="a Scryfall default_cards/oracle_cards bulk file (.json, .jsonl, .jsonl.gz) for every card's picture")
    ap.add_argument("--fetch-bulk", action="store_true", help="download default_cards into target/site-cache/ (once) and use it as --bulk")
    ap.add_argument("--card-cache", default=os.path.join(repo, "data/scryfall-cache"), help="codegen's per-card Scryfall payloads, for each pictured printing's page, artist and stamp")
    args = ap.parse_args()
    os.chdir(repo)

    rev = subprocess.run(["git", "rev-parse", args.rev], check=True, capture_output=True, text=True).stdout.strip()
    files = tree(rev, [LEDGER, CORE_INDEX, CARDS, UNPLAYABLE])

    # The ledger, in index order: release order by first printing.
    rows = []
    for m in ROW.finditer(files[LEDGER]):
        rows.append((int(m.group(1)), m.group(2), m.group(3), rust_string(m.group(4))))
    declared = int(re.search(r"pub static ROWS: \[Row; (\d+)\]", files[LEDGER]).group(1))
    if len(rows) != declared:
        sys.exit(f"ledger: read {len(rows)} rows, it declares {declared}")
    rows.sort()
    if [r[0] for r in rows] != list(range(len(rows))):
        sys.exit("ledger: indices are not 0..n")

    # Each set's count as the core index's per-set file states it.
    headers = {}
    for path, text in files.items():
        if path.startswith(CORE_INDEX + "/set_"):
            m = SET_HEADER.search(text)
            headers[m.group(2)] = int(m.group(1))

    # The pool.
    pool = {}
    for path, text in files.items():
        if not path.startswith(CARDS + "/") or not path.endswith(".rs"):
            continue
        if "card!(" not in text:
            continue  # cards/mod.rs and other non-card modules
        oid, sid, status, note = read_card(path, text)
        if oid in pool:
            sys.exit(f"{path}: oracle id {oid} also in {pool[oid][3]}")
        pool[oid] = (sid, status, note, path)

    never = {}
    for line in files[UNPLAYABLE].splitlines():
        if line.startswith("#") or not line.strip():
            continue
        oid, name, reason = line.split("\t")
        never[oid] = reason
        if oid in pool:
            sys.exit(f"unplayable {name} has a card file: {pool[oid][3]}")

    ledger_ids = {r[1] for r in rows}
    stray = [p[3] for o, p in pool.items() if o not in ledger_ids]
    if stray:
        sys.exit(f"{len(stray)} card files name an oracle id the ledger lacks: {stray[:5]}")
    for oid in never:
        if oid not in ledger_ids:
            sys.exit(f"unplayable {oid} not in the ledger")

    previous = {}
    prev_overview = os.path.join(args.out, "sets.json")
    if os.path.exists(prev_overview):
        with open(prev_overview, encoding="utf-8") as f:
            for s in json.load(f).get("sets", []):
                previous[s["code"]] = s
    prev_ids = {}
    if not args.bulk:
        for code in previous:
            p = os.path.join(args.out, "sets", f"set-{code}.json")
            if os.path.exists(p):
                with open(p, encoding="utf-8") as f:
                    for c in json.load(f)["cards"]:
                        if c.get("img"):
                            prev_ids[c["oracle_id"]] = c["img"]

    if args.fetch_bulk and not args.bulk:
        args.bulk = fetch_bulk(os.path.dirname(args.sets_cache))
    meta = scryfall_sets(args.sets_cache, not args.no_fetch)
    facts = printing_facts(args.card_cache)
    printings = bulk_printings(args.bulk, ledger_ids, facts) if args.bulk else {}
    dropped_ub, uri_count = 0, 0

    order, by_set = [], {}
    for idx, oid, code, name in rows:
        if code not in by_set:
            order.append(code)
            by_set[code] = []
        if oid in never:
            status, note, img = "never", never[oid], None
        elif oid in pool:
            sid, status, note, _ = pool[oid]
            img = sid
        else:
            status, note, img = "absent", None, None
        p = printings.get(oid)
        if p:
            img = p.get(code) or img or p.get("*")
        elif img is None:
            img = prev_ids.get(oid)
        if img is not None and not UUID.match(img):
            sys.exit(f"{name}: {img!r} is not a Scryfall id")
        # a picture only for the pool and the never-built list, never for
        # Universes Beyond: the printing's stamp, else its set or the set
        # the card is listed under
        shown = pictured(status, code, img, facts)
        if img and not shown and status in PICTURED:
            dropped_ub += 1
        img = shown
        _stamp, uri, artist, _pset = facts.get(img, (None, None, None, None)) if img else (None, None, None, None)
        card = {"name": name, "oracle_id": oid, "status": status}
        if note:
            card["note"] = note
        if img:
            card["img"] = img
            if uri:
                card["uri"] = uri
                uri_count += 1
            if artist:
                card["artist"] = artist
        by_set[code].append(card)

    for code, cards in by_set.items():
        if headers.get(code) != len(cards):
            sys.exit(f"set {code}: {len(cards)} ledger rows, its index file says {headers.get(code)}")
    if set(headers) != set(by_set):
        sys.exit(f"sets differ between ledger and core index: {set(headers) ^ set(by_set)}")

    keys = ("implemented", "partial", "unimplemented", "absent", "never")
    overview, totals = [], dict.fromkeys(keys, 0)
    os.makedirs(os.path.join(args.out, "sets"), exist_ok=True)
    for pos, code in enumerate(order):
        cards = by_set[code]
        counts = dict.fromkeys(keys, 0)
        for c in cards:
            counts[c["status"]] += 1
            totals[c["status"]] += 1
        m = meta.get(code) or {}
        entry = {
            "code": code,
            "position": pos,
            "name": m.get("name") or previous.get(code, {}).get("name") or code.upper(),
            "released": m.get("released_at") or previous.get(code, {}).get("released"),
            "total": len(cards),
            **counts,
        }
        overview.append(entry)
        with open(os.path.join(args.out, "sets", f"set-{code}.json"), "w", encoding="utf-8") as f:
            json.dump({"code": code, "cards": cards}, f, ensure_ascii=False, separators=(",", ":"))
            f.write("\n")
    # Remove set files no set names any more.
    for fn in os.listdir(os.path.join(args.out, "sets")):
        if not (fn.startswith("set-") and fn.endswith(".json") and fn[4:-5] in by_set):
            os.remove(os.path.join(args.out, "sets", fn))

    doc = {
        "rev": rev,
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "corpus": len(rows),
        "totals": totals,
        "sets": overview,
    }
    # One set per line: a regeneration diffs as the sets that moved.
    head = json.dumps({k: v for k, v in doc.items() if k != "sets"}, ensure_ascii=False, separators=(",", ":"))
    lines = ",\n".join(json.dumps(s, ensure_ascii=False, separators=(",", ":")) for s in overview)
    with open(prev_overview, "w", encoding="utf-8") as f:
        f.write(head[:-1] + ',"sets":[\n' + lines + "\n]}\n")

    with_img = sum(1 for cs in by_set.values() for c in cs if "img" in c)
    unnamed = [s["code"] for s in overview if s["name"] == s["code"].upper()]
    print(f"rev {rev[:12]}: {len(rows)} cards in {len(order)} sets, pool {len(pool)} card files")
    print("  " + ", ".join(f"{k} {v}" for k, v in totals.items()))
    print(f"  {with_img} cards with a picture id ({uri_count} with their Scryfall page), {dropped_ub} Universes Beyond printings by name only; {len(unnamed)} sets without a name {unnamed[:8]}")
    in_pool = totals["implemented"] + totals["partial"] + totals["unimplemented"]
    if in_pool != len(pool):
        sys.exit(f"pool {len(pool)} != implemented+partial+unimplemented {in_pool}")


if __name__ == "__main__":
    main()
