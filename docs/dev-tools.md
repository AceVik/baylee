# Native development tools

`baylee-tools` contains the repository's maintenance tools in Rust. It is a
small workspace binary, independent of the engine, card pool and renderer.
The `tools` profile optimises image/audio kernels and keeps panic unwinding
for native mount cleanup. It does not change the game's build profiles.
Python remains only in `tools/trainer`, whose PyTorch models and training
pipeline are intentionally unchanged.

```bash
cargo build --locked --profile tools -p baylee-tools
```

Run `cargo run --profile tools -p baylee-tools -- <group> …`, or `scripts/baylee-tools <group> …`.
The launcher passes its own checkout as `--root`, even with a shared prebuilt
binary. Direct calls infer the nearest Baylee workspace from the working
directory; `--root PATH` before the group selects it explicitly. Set `BAYLEE_TOOLS_BIN`
to an already-built binary to avoid Cargo startup in repeated checks or CI.
The helper's stdout belongs to its result (JSON, paths, checksums or TSV);
diagnostics go to stderr. CLI arguments, especially account passwords, must
not be copied into transcripts. No command is automatically authorised to
contact production or run a release.

| Former Python tools | Rust command group |
| --- | --- |
| `scripts/devctl/recording.py` | `recording record`, `encode`, `status`, `stop` |
| `scripts/shell/*.py` | `shell` (client driving, screenshots, layouts and input checks) |
| `scripts/release/ci_artifacts.py`, `pe_icon.py` | `release resolve`, `verify-run`, `write`, `verify`, `installer-sums`, `pe-icon` |
| `scripts/installers/make-art.py`, DMG settings | `installers art`, `dmg`, `check-dmg-layout` |
| `scripts/site/card_status.py`, `stamp.py`, `impressum_blob.py` | `site card-status`, `stamp`, `impressum-blob`, `check` |
| `scripts/forge/*.py` | `forge read-adventure-save`, `parse-friend-list`, `resolve-printings`, `to-deck-rows` |
| `scripts/llm/*.py` | `llm` (bounded card and test lanes) |
| `.claude/hooks/*.py` | `hooks card-test-debt`, `require-card-tests` |
| `tools/cardbatch/verdict.py` | `cardbatch <file> status` or `row <slug> <name>` |
| `art/music/prepare.py`, `originality.py` | `art prepare`, `originality` |
| `art/baylee/lantern.py` | `art lantern` (native glTF generation) |
| Server scripts' inline Python JSON readers | `baylee-server-json` (separate tiny native binary) |

Use each group's `--help` for the exact options. The existing shell entry
points for packaging, deployment and training remain shell scripts; they
invoke the new Rust tools where Python previously parsed or generated data.
Historical reports retain original command names as evidence of how their
measurements were made; they are not current instructions.

## Dev-control

The protocol, isolation rules and input timing are in
[the client documentation](client.md#driving-the-client-without-its-window).
The short agent guide is [.claude/skills/dev-control/SKILL.md](../.claude/skills/dev-control/SKILL.md).
Build the feature once, then launch an isolated, silent client. Keep the
shared Cargo lock and never run competing heavy builds or stop shared
Postgres. The helper sends input only to the loopback dev-control socket;
it does not automate OS input.

```bash
cargo run --profile tools -p baylee-tools -- recording record --duration-ms 3000 --fps 15 --format all --diff
```

```bash
cargo run --profile tools -p baylee-tools -- recording encode /tmp/baylee-recording-UUID --format all --diff
```

Raw PNGs and their timestamp ledger remain authoritative. MP4 (`preview-CADENCE.mp4` and 1 FPS `preview-step.mp4`), GIF/WebP, contact
sheets and pixel differences make a transition easier to inspect, but dropped
samples and ordinary animated pixels limit any flicker conclusion. Include a
paused control, inspect real frames, and retain incomplete/drop metadata.
Encoding and image utilities require only their documented native tools,
never a Python runtime or a hidden Python subprocess.

## Website and private data

```bash
cargo run --profile tools -p baylee-tools -- site card-status --rev origin/main --no-fetch
```

```bash
cargo run --profile tools -p baylee-tools -- site stamp --check
```

```bash
cargo run --profile tools -p baylee-tools -- site check
```

Impressum generation accepts private JSON and an output path outside every
repository path; it never logs the fields or their values. Use fictional
fixtures for tests. Do not commit personal data or generated private blobs.
The public script/feedback decoder must retain the same blob version and
constants. The public card catalog is read from a committed revision, keeps
its ledger/coverage consistency checks and excludes disallowed images.

## Server JSON helper

`baylee-server-json` parses bounded health/info JSON from stdin. Counts must
be nonnegative integers. Missing, malformed or oversized state returns an
error, which deployment/reboot scripts treat as unknown and keep waiting.
`local_running` falls back to `running` only when that field is absent.
The deploy stage uses the freshly built helper for preflight and installs it
at `/opt/baylee/bin/baylee-server-json` after preparation succeeds, with the
agent/engine binaries. Custom installations set `BAYLEE_SERVER_JSON` in both
scripts. Install the native helper before installing the new reboot script.
No Cargo or Python is invoked by a health check.

## Tests

```bash
cargo test --locked -p baylee-tools --all-targets
```

These tests replace the old Python unittest suites and participate in the
workspace's debug and optimized test gates. Release tests cover provenance
and checksums; website tests cover catalog/image/privacy guards; tool tests
cover untrusted parsing, bounds and output semantics. Test fixtures never
publish releases or call live model APIs. Native macOS/Windows installer
checks still require the matching OS and SDK.

Native Finder metadata uses `ds_parser` and `alias_record` under
[Blue Oak 1.0.0](https://blueoakcouncil.org/license/1.0.0); their license notice
must accompany any redistribution of those tools. The dependency allowlist
records this tooling-only addition. The font rasteriser reuses `ab_glyph`
already in the workspace graph and reads only the bundled OFL fonts.

For the first rollout from a Python-based deploy script, the old running
script may build only its old package list before installing the new script.
After that rollout, run the updated `baylee-deploy stage <same deployed rev>`
once, or build/install `baylee-server-json` before enabling the updated reboot
timer. Until the helper is installed, missing health parsing fails closed;
it cannot accidentally restart a gateway with an unknown game count.

## Migration checks (14 October 2026)

The migration removes 41 Python files; the 15 files under `tools/trainer`
remain unchanged. The final local workspace gate passed 14,668 tests with 24
skipped; the feature and wasm gates also passed. The native tools passed 250
optimized tests plus two server-reader tests, Clippy, Rust 1.97.1 and
`cargo deny`. The opt-in macOS DMG test was run separately: it built, mounted,
verified and detached its own temporary image, including background, volume
icon, Finder layout and app bundle. Windows installer execution still belongs
to the Windows CI runner.

Offline catalog output matched all 382 sets and 33,694 cards from the former
Python generator, comparing JSON values and array order while ignoring only
the generated timestamp. A 99-frame real-client recording preserved all
frames in GIF/WebP and reproduced the former full-resolution RGB difference
values within 1e-12. A fresh isolated, silent client capture completed five
frames without drops through the native helper.

Asset migration leaves shipped artwork and audio untouched. The native
lantern generator emits a candidate GLB; rebuilding Blender authoring files
and a matching Cycles render remains a separate Blender step, as described in
[the asset guide](../art/baylee/README.md#editable-blender-source). Orchestra
selection, DSP fixtures and generated runtime definitions passed, but the
complete sample bank was not regenerated because its raw source cache was
empty. Live model APIs and production deployments were not exercised by this
migration.
