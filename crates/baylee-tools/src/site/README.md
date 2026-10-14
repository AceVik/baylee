# Native site tools

All commands run through `cargo run --profile tools -p baylee-tools -- site`:

| Previous entry point | Native command |
| --- | --- |
| `scripts/site/card_status.py` | `site card-status [--rev REV] [--out PATH] [--sets-cache PATH] [--no-fetch] [--bulk PATH] [--fetch-bulk] [--card-cache PATH]` |
| `scripts/site/stamp.py` | `site stamp [--check]` |
| `scripts/site/impressum_blob.py` | `site impressum-blob PRIVATE_JSON [OUT_BIN]` |
| Site/static privacy checks | `site check` |
| `cd web && python3 -m http.server 8000` | `site serve --port 8000 [--bind 127.0.0.1]` |
| Three Python regression files | `cargo test -p baylee-tools site::` |

`card-status` reads a resolved commit with `git archive`, validates the complete
ledger/pool/core-index/exclusion relationship before writing, and keeps the
existing overview and lazy `sets/set-<code>.json` format. Bulk input is read one
line at a time, plain or gzip; only winning printing candidates and their facts
are retained. Picture eligibility and the seventeen Universes Beyond fallback
codes match the previous tool. A previous printing can never make an absent or
Universes Beyond card eligible for a picture. Unrelated output files are kept;
only obsolete generated `set-*.json` files are removed.

`stamp` updates CSS dependencies first and hashes the resulting CSS for HTML.
Its check mode writes nothing. `check` runs the original catalog, accessibility,
CSP, legal image presentation, static Impressum, robots, gitignore and privacy
notice guards without making a network request.

`impressum-blob` retains the browser/service v2 envelope and shuffled Unicode
code-point pairs. Every field is trimmed, nonempty and at most 64 Unicode scalar
values. It uses OS randomness, unbiased shuffling, the same 64-attempt readable
substring rejection, a random temporary file and atomic replacement. Existing
symlinks and `..` are resolved before enforcing the outside-repository fence,
which is checked again after creating the parent directory. Errors omit source
values, parser excerpts and paths. Only fictional fixtures are used by tests.

`serve` serves `root/web`, including `/site/`, on a loopback IP only. It supports
GET and HEAD, an 8 KiB request-header limit, a five-second total header deadline,
MIME types and `Cache-Control: no-cache`. It has no directory listings or request
logs. Canonical paths must stay within the web root; private directories, hidden
files, blob files and Impressum input filenames are also refused.

## Migration validation

The suite has **137** tests and retains all **107** original test functions. Additional tests cover
catalog source inconsistencies, printing priority, bad bulk payloads, CSS hash
order, negative page fixtures, malicious/truncated blobs, Unicode limits,
symlink fences, browser decoding and the preview HTTP server. Browser tests run
the actual shipped JavaScript with Node if installed, as the former tests did;
they make no network requests. Preview HTTP tests use in-memory requests and
responses, without opening sockets. Three captured Python reference vectors check a
v2 blob, a partial card definition, and bulk selection/facts without requiring
Python at test time.

A full offline baseline from revision `7f054ef5541c` is available during this
migration at `/tmp/baylee-site-legacy-offline`: 33,694 cards / 382 sets / 3,318 pool
files; totals 3,219 implemented, 51 partial, 48 unimplemented, 30,363 absent and
13 never. Generate the native comparison into a separate temporary directory
with `--rev 7f054ef5541c --no-fetch`, a nonexistent `--sets-cache`, and a
nonexistent `--card-cache`. Compare JSON values recursively, ignoring only
`sets.json.generated_at`; object-key ordering is not significant. Preserve the
set array and card array order. No shipped JSON needs to be overwritten for this
comparison.

To reproduce the legacy command after deletion, read its source with
`git show 7f054ef5541c:scripts/site/card_status.py`, execute it with `__file__`
set to the old repository-relative script path, and pass explicit temporary
output/cache paths and `--no-fetch`. The old main derives its repository from
`__file__`, so running a file extracted directly into `/tmp` would point it at
the wrong repository. Builds, test execution, manifests, shared docs and commits
are serialized by the parent task.
