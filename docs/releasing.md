# Releasing

The PM owns releases and the workspace version (`docs/dictionary.md`, Revier).

## What a release is

A push to `main` builds the desktop packages once, alongside the CI checks.
A tag `v<version>` on that green commit makes `.github/workflows/release.yml`
promote those exact archives, sign them and publish a GitHub Release. The tag
never rebuilds the client. The release has one
archive per desktop target (Linux, Windows x86_64/aarch64 and macOS aarch64),
each with a `.sha256` and a signature (`.sig`, §"Signing") beside it. Every
archive holds a permanent launcher (`baylee-client` / `baylee-client.exe`),
the real client (`baylee-runtime` / `baylee-runtime.exe`), `assets/` (the fonts load from there at run time,
together with their licences),
`LICENSE`, `NOTICE` and a `README.txt`. `scripts/package-client.sh` builds the
archive and also runs locally. Beside each archive stand its installers (a
dmg, a setup `.exe`, an AppImage and a `.deb`, §"Installers"), made from the
same tree in the same CI job.

The binaries are built with `--profile dist` (`Cargo.toml`), which is
`release` plus line tables and no strip, so a crash report keeps file and line.
Fat LTO was measured and does not fit: it needs 17 GB at link time, and a free
runner has at most 16 GB. On x86_64 the target CPU is `x86-64-v2`.

The builds are **not code-signed**. Signing and notarisation cost money, so
that is the owner's decision. Until then macOS and Windows warn on first
start, and each `README.txt` explains how to get past the warning.

## Version numbers

`workspace.package.version` is the one version. `baylee-build` stamps it into
every binary together with the commit and the CI run.
While the version is `0.x`:

- **minor** (`0.1 → 0.2`): anything an older build cannot talk to or read:
  a `VIEW_VERSION` bump, a protocol change, or a saved-data format change
  (decks, preferences, replays, `CardIndex`).
- **patch** (`0.1.0 → 0.1.1`): everything else.

## Cutting one

1. On `main`, bump `version` in `[workspace.package]` in its own commit
   (`chore(release): 0.2.0`) and push it.
2. Wait for `ci` to go green on that commit, including `packages`. The workflow
   refuses a tag without a successful **push to this repository's main** CI run
   at that exact SHA in which every job of `REQUIRED_JOBS`
   (`scripts/release/ci_artifacts.py`) ran and passed, and a tag that differs
   from the workspace version. A green PR run is insufficient, and so is the
   run of a docs-only push, which skips the release jobs (§"CI"); the version
   bump touches `Cargo.toml`, so its own run is always a full one.
3. `git tag -a v0.2.0 -m "Baylee 0.2.0" && git push origin v0.2.0`.

To dry-run, start `release` by hand (`gh workflow run release.yml`). It does
the same builds and uploads them as workflow artifacts, versioned
`<version>-dev.<commit>`, and publishes nothing.

## Build reuse and caches

`.github/workflows/client-packages.yml` is the one packaging recipe. Main CI
links the two shipped binaries (`baylee-client`, `baylee-launch`) in `dist` on
each of the five shipping targets, with `--workspace` as the selection so their
dependencies' features unify exactly as a whole-workspace build would, checks
every target's code (`--all-targets`), and uploads the client/launcher archives.
The Intel macOS link check stays in its own parallel job; the link of the other
binaries on all five platforms is the nightly `build` matrix. PR and dispatched
runs cannot supply release archives.

Each package artifact includes its archive, its installers, a SHA-256 checksum
file for each and a manifest (schema 2) naming repository, commit, source CI
run, version, architecture and every file's digest. Promotion
checks the source run through GitHub's API, requires all five unexpired artifacts,
and verifies every file and manifest before signing. It rejects unexpected
files and wrong hashes; artifacts from another commit are never a fallback.
The build number displayed by the client belongs to the **source CI run**, not
the later signing run. Promotion preserves the archive bytes unchanged.

Package artifacts are kept for **14 days**. If they expire, rerun the original
main CI run at that commit, wait for success, then rerun the release workflow.
Commits from before this packaging workflow do not have promotable artifacts.
A manual release dry run still builds `<version>-dev.<commit>` packages and
publishes nothing; it does not masquerade as a trusted main push. To test the
promotion/signing path after a green main CI without cutting another release:
`gh workflow run release.yml --ref main -f promote-ci=true`. This reuses the
workspace-version archives; the publish job remains tag-only.

The five desktop package builds cache compiled dependencies. Cache keys
separate runner OS/target and compiler configuration. **Only a push to main
writes any cache**; every other run (a PR, a dispatch, the nightly, a dry run)
restores main's. A PR's own cache would live under `refs/pull/N/merge`, which
no other run can read, and on 08.10.2026 those PR writes plus the PR platform
matrix had pushed the repository past its 10 GB, so most jobs restored nothing
(§"CI").

The first debug-test partition writes a shared cache consumed read-only by
Clippy, feature tests, validation and the signer. One writer avoids an
immutable key being filled first by an incomplete Clippy-only build. Workspace
crates are not added to every compiler cache, because larger duplicated caches
would evict each other. Exact
finished client packages instead live in the 14-day artifact store. Cache
eviction or compiler/profile changes can still cause cold builds. No signing
key enters a cache, and reuse never skips Cargo's freshness checks.
The [rust-cache inputs](https://github.com/Swatinem/rust-cache#example-usage) and
[GitHub cache scoping](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache)
describe the underlying behavior.

`ci-release` inherits release optimization (`opt-level=3`) and explicitly keeps
debug assertions and overflow checks disabled, but disables LTO and uses 16
codegen units. This avoids whole-program optimization for every test executable.
It is a release-semantics regression check, not a claim to test the exact shipping
link configuration: `dist` packages retain thin LTO and one codegen unit.
See [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html).

Baseline for beta.3: CI's optimized test job took 55 minutes (actual tests: 110
seconds), followed by 33.5 minutes for the slowest desktop release build and
4.2 minutes for signing. The new tag path removes the second desktop compilation.
Cold and warm timings still need comparison on GitHub; no fixed speedup is promised.

Local checks for pipeline changes:

```sh
python3 -m unittest discover -s scripts/release/tests -v
cargo test --locked -p baylee-update --test sign_script
actionlint
shellcheck scripts/package-*.sh scripts/installers/*.sh scripts/release/*.sh
```

## CI

`.github/workflows/ci.yml` runs on main pushes, pull requests, nightly and by
hand. Since 08.10.2026 a change **lands on main after the local gates without
waiting for CI**: `scripts/land.sh` runs `gate.sh`, `gate-features.sh` and
`gate-wasm.sh` under the machine's cargo lock and then pushes the gated commit
to main, only as a fast-forward (it never rebases, merges or forces; `--check`
stops before the push). CI green matters at a release, which needs the main
push run of the tagged commit green (§"Cutting one").

| Trigger | Runs |
| --- | --- |
| main push (code or CI changed) | everything: fmt, clippy, test ×2, test-release, features, validate, wasm, web-feedback, deny, audit, bench, msrv, macos-intel, packages ×5 |
| main push (docs only) | fmt, test ×2 (tests read docs); not taggable |
| pull request | fmt, and as their inputs changed: clippy, test ×2, wasm, validate, deny, web-feedback |
| pull request labelled `ci:full`, or changing CI | as a main push (its packages are never promotable) |
| nightly | as a main push without packages, plus the five-platform `build` matrix |
| `workflow_dispatch` | `full` (default on) as a main push; `platforms` adds the matrix |

`scripts/ci-changes.sh` decides what a diff touches (try it with
`git diff --name-only origin/main | scripts/ci-changes.sh`). Every job has a
`timeout-minutes` of about 1.5× its measured p95. The debug suite runs in two
nextest partitions (`--partition count:i/2`), each compiling for itself.

Measured before this layout (30 main pushes and 31 PRs, 06.–08.10.2026,
minutes; queueing was under 3 min at p95):

| | PR p50 / p95 | main push p50 / p95 |
| --- | --- | --- |
| whole run (first attempt, to the last job) | 27 / 40 | 56 / 69 |
| test (debug) | 19 / 24 | 19 / 24 |
| test-release | 22 / 39 | 23 / 40 |
| build matrix, macOS row | 24 / 29 | — |
| features | 18 / 20 | 19 / 20 |
| package aarch64-apple-darwin | — | 55 / 69 |
| package x86_64-pc-windows-msvc | — | 37 / 51 |

Three jobs (bench, msrv, features) hung on 07.10.2026 for 228, 361 and 360
minutes, all in `apt-get update` against a mirror that stopped answering; apt
is now bounded in `.github/actions/linux-deps` and every job has a timeout.

## Installers

The owner asked on 06.10.2026 for installers like other desktop apps have.
They stand **beside** the archives, never instead of them: the updater only
ever downloads `baylee-client-<version>-<target>.{zip,tar.gz}`, checks its
`.sig` and the signed root folder's name (§"Desktop launcher and
recovery"), and an installed client updates itself from there. So the
installers get no `.sig`; each has a `.sha256` beside it, and the publish
job writes a table of them into the release notes
(`ci_artifacts.py installer-sums`, which also refuses a release missing one).

| Target | Installer | Built with |
| --- | --- | --- |
| aarch64-apple-darwin | `Baylee-<version>-aarch64.dmg` | dmgbuild 1.6.7 (pip, hash-pinned) |
| x86_64 / aarch64 Windows | `Baylee-Setup-<version>-{x64,arm64}.exe` | Inno Setup 7.1.0 |
| x86_64 / aarch64 Linux | `Baylee-<version>-{x86_64,aarch64}.AppImage` | appimagetool 1.9.1, type2 runtime 20251108 |
| x86_64 / aarch64 Linux | `baylee_<version>_{amd64,arm64}.deb` | the runner's `dpkg-deb` |

In `client-packages.yml`, after `package-client.sh`,
`scripts/installers/tools.sh` fetches the tools (each pinned by version and
SHA-256), `scripts/package-installers.sh` wraps the staged tree, and
`scripts/release/check-installers.sh --install` installs each one on the
throwaway runner, checks it and removes it again: the dmg's app is copied to
`/Applications` and started from there until it logs `assets from
/Applications/Baylee.app/…`; the setup installs for the runner's user and
uninstalls; the `.deb` goes in with `apt`, which proves its dependencies
resolve. Without `--install` the script only mounts, extracts and lists,
which also works on a Mac. The pictures (dmg background, `.ico`, Linux icon)
are our own, drawn by `scripts/installers/make-art.py` from the brand icon
and Alegreya Sans, and committed.

The app icon is one rounded square (quarter-superellipse corners,
transparent outside) at every size, all written by that script: the
bundle's `baylee.icns` on Apple's 824/1024 grid, the Windows `.ico`
(16–256 px), the Linux hicolor PNGs (16–512 px) and the 128 px window icon.
Where each platform finds it:

- **macOS**: `CFBundleIconFile` in the bundle; a `cargo run` client sets the
  same `.icns` on the Dock itself (`app_icon.rs`).
- **Windows**: both executables carry the `.ico` as resource 1, embedded by
  `build.rs` of `baylee-client` and `baylee-update` through `winresource`
  and the SDK's `rc.exe` (a build-dependency on a Windows host only; a
  cross-build from elsewhere warns and carries none). That is Explorer's,
  the taskbar's and the title bar's icon: the running client loads it for
  its window (`window_icon.rs`) and claims the AppUserModelID
  `AceVik.Baylee`, which the setup also writes on its shortcuts, so the
  window joins a pinned Baylee button rather than opening a second one
  whose pin would bypass the launcher. The launcher starts the client with
  `CREATE_NO_WINDOW`, so no console window opens beside it. The setup, its
  uninstaller, the shortcuts and Settings → Apps use the `.ico` file.
  `check-installers.sh` reads both executables' resource tables
  (`scripts/release/pe_icon.py`) before it installs anything.
- **Linux**: `baylee.desktop` (`Icon=baylee`, `StartupWMClass=baylee`) and
  the hicolor sizes in the `.deb` and inside the AppImage (plus its
  `.DirIcon`). The window's Wayland `app_id` and X11 `WM_CLASS` are
  `baylee`, which is how a dock pairs it with the entry; on X11 the window
  also sets the icon itself. An AppImage shows in a menu or dock only once
  something integrates it (appimaged, AppImageLauncher, Gear Lever).

What each installer does, and whether the client then updates itself
(`launch::placement` decides, by trying to create a file beside the
original package, or beside the AppImage file):

- **dmg**: the classic window, Baylee.app and a link to `/Applications`. The
  app keeps package-client.sh's ad hoc signature; a copy dragged to
  `/Applications` is writable for an administrator account (the default),
  so it updates itself. Started straight from a downloaded image, macOS
  translocates it and it still updates into its per-user state, keyed by
  `/Volumes/…/Baylee.app`; an image without quarantine is read-only, so it
  only links and offers the move to Applications.
- **Setup**: per user, `%LOCALAPPDATA%\Programs\Baylee`, no administrator
  rights and no UAC prompt, a Start-menu entry, an optional desktop icon and
  an uninstall entry under HKCU (Settings → Apps). The folder is the
  player's, so it updates itself. The uninstaller leaves
  `%LOCALAPPDATA%\baylee` (downloaded updates and the card-image cache,
  shared by every installation of this user).
- **.deb**: the tree in `/opt/baylee`, `/usr/bin/baylee`, a `.desktop` entry
  and icon. `/opt/baylee` is root's, so the client only says a release is
  out and links to it; the player installs the new `.deb`. Version
  `0.1.0-beta.5` becomes `0.1.0~beta.5`, which dpkg sorts before `0.1.0`.
- **AppImage**: the same tree in one file, using the system's libraries
  exactly as the tarball does (nothing is bundled; its README lists them).
  The image is mounted read-only at a fresh `/tmp/.mount_…` each start, so
  the launcher keys its state on the image file (`$APPIMAGE`, believed only
  with `$APPDIR` around the launcher; `launch::appimage_of`): one state
  folder per image, and where the image's folder is writable (`~/Apps`,
  say) it updates itself into that state like the other packages. The
  image file is never rewritten (only the archive is signed). It carries no
  zsync update information: the client's updater is the update path.

Installing a newer package by hand over the original (or replacing an
unpacked archive in place) wins over an older downloaded generation: the
launcher starts the original whenever its own version is at least that of
`current.json` (`launch::chosen`).

No Intel macOS package: the shipping matrix has no `x86_64-apple-darwin`
dist build. A universal app would not be cheap either, because the updater
asks for the archive of its compiled target, so an Intel slice needs its own
archive, matrix row (about half an hour per main push) and updater tests.

**What signing would add.** macOS: an Apple Developer Program membership
("The Apple Developer Program is 99 USD per membership year",
developer.apple.com/programs/enroll, read 06.10.2026) buys a Developer ID
certificate; signing the app and the dmg with it and notarising both lets
Gatekeeper open the app without the trip to Privacy & Security. Windows: an
Authenticode certificate from a certificate authority or Microsoft's
Artifact Signing service (an Azure subscription; its price page names none
we could read) signs the setup and the executables; SmartScreen still warns
until the signature has earned reputation. Both are the owner's call, and
neither changes the update signature, which stays our own Ed25519 key.

## Signing

The client installs updates by itself (#326, `docs/client.md` §"Updating"),
and only an archive whose Ed25519 signature verifies against a key compiled
into it (`crates/baylee-update/src/sign.rs`, `TRUSTED_KEYS`). So every
archive gets a `<archive>.sig` (base64 of the 64-byte signature) before it is
published. The `.sha256` beside it only catches a broken download; anyone who
can replace an archive can replace its checksum.

In `release.yml` the `sign` job downloads the promoted archives and runs
`scripts/release/sign-archives.sh`, which signs them with the seed in the
repository secret `BAYLEE_UPDATE_SIGNING_KEY` and then verifies each
signature against the compiled keys. A secret that is not the client's key
therefore fails the release, not every player's updater. On a tag, a missing
secret fails the job and nothing is published; `publish` also refuses an
archive without its `.sig`. A dry run without the secret warns and goes on
unsigned. Only the build step sets `BAYLEE_RELEASE_BUILD=1`, which is what
lets a client replace itself; a build made anywhere else only checks and
links.

`crates/baylee-update/tests/sign_script.rs` runs the script against a test
key and holds the workflow's wiring; `cargo run -p baylee-update --bin
baylee-update-sign -- verify <archive>` checks a published archive by hand.

### The key, once

The owner does this once, on their own machine:

1. `cargo run -p xtask -- update-key` writes a new seed to
   `~/.config/baylee-release/update-signing.key` (mode `0600`, never
   overwritten) and prints the public key. The seed of 27.09.2026 is there
   already, and its public key is the one in `TRUSTED_KEYS`.
2. Store the file's content as the repository secret:
   `gh secret set BAYLEE_UPDATE_SIGNING_KEY < ~/.config/baylee-release/update-signing.key`.
3. Back the file up somewhere offline (a password manager, an encrypted
   stick). Lost, no client that has only this key can be updated
   automatically again; its players have to download the next release by
   hand once.

The seed never enters the repository, a log or an issue.

### Rotating the key

1. Make a new key with `update-key --out <path>` and **append** its public
   key to `TRUSTED_KEYS`; keep the old one.
2. Release that build signed with the **old** key, so every installed client
   takes it.
3. Once players have had time to update, switch the secret to the new seed
   and release; a later release may drop the old key from the list.

A key that leaked is removed from the list at once instead, and players on
older builds update by hand once.

## Desktop launcher and recovery

`package-client.sh` requires both the Bevy client and `baylee-launch` in
its input directory. The main CI package workflow builds both with the dist profile.
On macOS both live in `Baylee.app/Contents/MacOS`; `CFBundleExecutable`
continues to name `baylee-client`, now the launcher. The runtime is signed
before the enclosing bundle. Automatic updates never alter this original
bundle, its resource seal, or the normal launch executable.

The launcher stores each user's state under `$XDG_STATE_HOME/baylee` (or
`$HOME/.local/state/baylee`) on Unix and `$LOCALAPPDATA/baylee` on Windows.
A hash of the canonical path of the original package isolates installations.
Under macOS App Translocation that is the bundle the player unpacked, as the
Security framework names it, not the randomised read-only mount it runs
from, so in-place and translocated starts share one state
(`launch::in_state_root_of`, `docs/client.md` §"Updating"). An AppImage,
mounted at a fresh `/tmp/.mount_…` each start, is keyed by its image file
(`$APPIMAGE`, believed only with `$APPDIR` around the launcher;
`launch::appimage_of`), and installs updates where that file's folder is
writable. The image file itself is never rewritten: updates are generations
in the state directory, as for every package, because only the archive is
signed.
Moving or renaming the original package starts a separate state directory.
The original client always remains in the package; downloaded complete
release trees live in `versions/<UUIDv7>` in the state directory.

The archive's **signed internal root name** must exactly equal the expected
`baylee-client-<version>-<target>` name. A valid signature for another release
or CPU architecture is insufficient. Checksums and release metadata alone
cannot establish that identity.

`activation.json` records intent before the staged tree moves. Files are
flushed, the whole tree is renamed into a new generation, and `current.json`
is atomically replaced to select it. The launcher starts the selected
generation unless its own package is at least as new (its compiled version
against `current.json`'s, `launch::chosen`): a newer package installed by hand
over the original then starts, and the session names the runtime it started.
The permanent launcher resumes pending
activation before starting a client. If recovery cannot finish, it retains
its intent and payload and starts the previously selected client. The
original launch path is never part of the transaction. These guarantees
cover process termination; hardware/power-loss durability also depends on
the filesystem (directory flushes are best effort, unavailable via this
implementation on Windows).

Permanent OS file locks serialize mutations. An admission lock and shared
lifetime leases in both launcher and client prevent simultaneous launches,
including when a launcher is killed but its child remains alive. A session
token rejects a late child of a superseded launcher. Only while holding an
exclusive lifetime lease does the launcher prune old generations; it keeps
the selected generation and its predecessor. Cleanup failures are retried
at the next launch. The untouched original package is retained separately.
A read-only original installation still starts, including an already selected
payload, but its session disables automatic installation: updates would not
write that folder, but whoever can is the one to manage it. The session names
the folder and the error (`launch::Blocked`), and a macOS client offers to
move itself to `~/Applications`. A translocated macOS app is not read-only
in this sense (it is the player's own download) and installs updates. The
session's fields after `writable` are `#[serde(default)]`: the launcher is
permanent in every package, so a runtime must read a session from every
older launcher. A directly started runtime also cannot auto-install; use the
normal packaged launch path.

The old rename journal code remains for explicit legacy recovery. Rollback
has a persisted direction and progress; failed reverse renames retain the
journal and required files, and neither staging nor a new legacy transaction
may erase an outstanding recovery. It is no longer the native installation
path. Packages predating the launcher must be replaced manually once; a
launcher protocol change likewise requires a new manual package for now.

Focused regressions: `cargo test -p baylee-update --all-targets` includes
actual launcher/runtime subprocesses, killed activation processes,
concurrent activation and startup, an orphaned runtime, read-only Unix
installations, bounded cleanup, signed version/architecture replay, and
retryable legacy rollback. Run this on each desktop OS. Before release,
additionally smoke-test the real packaged Bevy app via Finder/Explorer/the
normal Linux entry, since a test runtime cannot prove GUI activation,
platform security dialogs, or the macOS bundle's runtime behavior.
