# Native development tools

The independent PC/GPU dashboard is documented in
[`workstation-monitor.md`](workstation-monitor.md). It runs alongside training
with low scheduling priority and serves live metrics to desktop/mobile browsers.

`baylee-tools` contains the repository's maintenance tools in Rust. It is a
small workspace binary, independent of the engine, card pool and renderer.
The `tools` profile optimises image/audio kernels and keeps panic unwinding
for native mount cleanup. It does not change the game's build profiles.
Python remains only for PyTorch models, tensor execution and their direct tests
in `tools/trainer`. Training orchestration, packing, admission, monitoring and
other operational work use `baylee-train-ops` in Rust (see
[`trained-ai.md`](trained-ai.md#native-experiment-control-and-ppo-admission));
the developer tasks that were shell scripts are `cargo dev` (below).

```bash
cargo build --locked --profile tools -p baylee-tools
```

Run `cargo tools <group> …` (the alias in `.cargo/config.toml` for
`cargo run --locked --profile tools -p baylee-tools --`). It infers the
nearest Baylee workspace from the working directory; `--root PATH` before the
group selects one explicitly.
The helper's stdout belongs to its result (JSON, paths, checksums or TSV);
diagnostics go to stderr. CLI arguments, especially account passwords, must
not be copied into transcripts. No command is automatically authorised to
contact production or run a release.

| Former script | Rust command |
| --- | --- |
| `scripts/devctl/recording.py` | `cargo tools recording record`, `encode`, `status`, `stop` |
| `scripts/shell/*.py` | `cargo tools shell` (client driving, screenshots, layouts and input checks) |
| `scripts/release/ci_artifacts.py`, `pe_icon.py` | `cargo tools release resolve`, `verify-run`, `write`, `verify`, `installer-sums`, `pe-icon` |
| `scripts/release/check-installers.sh` | `cargo tools release check-installers [--install] TARGET VERSION` |
| `scripts/release/sign-archives.sh` | `baylee-update-sign release DIR` (crates/baylee-update) |
| `scripts/package-client.sh`, `package-installers.sh` | `cargo tools package client`, `package installers` |
| `scripts/installers/make-art.py`, `tools.sh`, DMG settings | `cargo tools installers art`, `tools`, `dmg`, `check-dmg-layout` |
| `scripts/site/card_status.py`, `stamp.py`, `impressum_blob.py` | `cargo tools site card-status`, `stamp`, `impressum-blob`, `check` |
| `scripts/forge/*.py` | `cargo tools forge read-adventure-save`, `parse-friend-list`, `resolve-printings`, `to-deck-rows` |
| `.claude/hooks/*.py` | `cargo tools hooks card-test-debt`, `require-card-tests` |
| `art/music/memorial/prepare.py`, `prepare_bank.py` | `cargo tools art memorial score`, `bank` (see `art/music/memorial/README.md`) |
| `art/music/prepare.py`, `originality.py` | `cargo tools art prepare`, `originality` |
| `art/baylee/lantern.py` | `cargo tools art lantern` (native glTF generation) |
| `scripts/server/baylee-deploy`, `baylee-invite`, `baylee-reboot` and their JSON reader | `baylee-deploy`, `baylee-invite`, `baylee-reboot` (crate baylee-ops) |
| `scripts/macos-runner/runner.sh` | `baylee-macos-runner` (crate baylee-runner) |
| `scripts/gate*.sh`, `land.sh`, `ci-changes.sh`, `deploy.sh` | `cargo dev gate [rules\|touched\|features\|shipped-features\|wasm]`, `land`, `ci-changes`, `deploy` |
| `scripts/mobile/*.sh` | `cargo dev android [env]`, `ios-sim`, `serve-lan` |
| `scripts/test-keyring-linux.sh`, `test-central-*.ps1` | `cargo dev keyring-linux`, `central db\|integration\|client` |

`cargo dev` (crates/baylee-dev) is the developer tasks' binary: small, with no
workspace dependency, so a gate compiles it in seconds. The cheap-model card
lanes (`scripts/llm/`, `tools/cardbatch/`, `baylee-tools llm` and `cardbatch`)
were removed on 20.10.2026; cards are written by the batch's author model and
reviewed by another (`docs/cards.md`). Python remains only in `tools/trainer`.
Historical reports retain original command names as evidence of how their
measurements were made; they are not current instructions.

Current landing runs the full and desktop/server feature gates. Android,
iOS and browser/wasm support are coming soon: mobile commands and
`cargo dev gate wasm` remain opt-in experiments outside CI and release
requirements (`docs/releasing.md`). Development features run only locally;
CI's `cargo dev gate shipped-features` only checks that test hooks cannot
reach shipped feature graphs.

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

## The server's commands

`baylee-deploy`, `baylee-invite` and `baylee-reboot` are the binaries of crate
`baylee-ops`, installed into `/usr/local/sbin` by every stage. They read the
gateway's health and info JSON themselves: counts must be nonnegative
integers, and missing, malformed or oversized state counts as unknown, so the
deploy and the reboot keep waiting. `local_running` falls back to `running`
only when that field is absent. Their tests (`crates/baylee-ops/tests`) stand
in the machine's commands on PATH and check what would run.

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

The server scripts became `baylee-ops` on 20.10.2026. A server still running
the shell `baylee-deploy` must take its next deploy by hand
(`BAYLEE_DEPLOY_HOST=… cargo dev deploy`): the shell script's `watch` would
stage a newer release with its old package list and look for scripts that are
gone. From then on the timer runs the installed binary.

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
