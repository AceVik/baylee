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
the real client (`baylee-runtime` / `baylee-runtime.exe`), the seat bridge
(`baylee-seat` / `baylee-seat.exe`, which seats a language model in a room;
without it, as in beta.6, the room's Language model is off), `assets/` (the fonts load from there at run time,
together with their licences),
`LICENSE`, `NOTICE` and a `README.txt`. `cargo tools package client` builds the
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

Pre-release names follow SemVer §11, and the updater compares by it
(`baylee_update::version`): a purely numeric identifier sorts **below** any
alphanumeric one, and alphanumeric ones compare as text. So after
`0.1.0-beta.8a` and `0.1.0-beta.8b`, a `0.1.0-beta.9` would be older than both
and never offered to those installs, and lettered names break at ten
(`10a` sorts below `9a`). The owner requested `beta.9a` on 28.10.2026, so
this release raises the patch and is `0.1.1-beta.9a`. It sorts above every `0.1.0` prerelease and
Central's previous `0.1.0` service package. To switch back to numeric
prereleases afterwards, raise the patch again (for example `0.1.2-beta.1`);
`0.1.1-beta.10` would sort below this alphanumeric prerelease.

## Cutting one

1. On `main`, bump `version` in `[workspace.package]`, rewrite the API
   documents (`cargo dev api`: they name the version, and the gate's `api`
   step refuses stale ones) and measure the card status
   (`cargo tools site card-status`, which rewrites `web/site/data`), all in
   one commit (`chore(release): 0.2.0`), and push it. The website
   (Central's `web/`) reads the card status of the newest release from that
   release's tag, as it reads the downloads from its assets, so the numbers a
   visitor sees are always those of the client the release ships. The release
   workflow refuses a tag whose status was measured at another pool
   (`cargo tools site card-status-fresh`: the measured commit must be an
   ancestor of the tag, and no card, ledger, core index or
   `data/unplayable.tsv` may have changed since). A dry run only warns about
   a stale status, since between releases it always is.
2. Wait for `ci` to go green on that commit, including `packages`. The workflow
   refuses a tag without a successful **push to this repository's main** CI run
   at that exact SHA in which every job of `REQUIRED_JOBS`
   (`baylee-tools release`) ran and passed, and a tag that differs
   from the workspace version. A green PR run is insufficient, and so is the
   run of a docs-only push, which skips the release jobs (§"CI"); the version
   bump touches `Cargo.toml`, so its own run is always a full one.
3. `git tag -a v0.2.0 -m "Baylee 0.2.0" && git push origin v0.2.0`.

To dry-run, start `release` by hand (`gh workflow run release.yml`). It does
the same builds and uploads them as workflow artifacts, versioned
`<version>-dev.<commit>`, and publishes nothing.

## Build reuse and caches

`.github/workflows/client-packages.yml` is the one packaging recipe. Main CI
links the three shipped binaries (`baylee-client`, `baylee-launch`,
`baylee-seat`: `cargo build-client`) in `dist` on each of the six shipping
targets, with `--workspace` as the selection so their dependencies' features
unify exactly as a whole-workspace build would, checks every target of the
client crates that carry a platform `cfg` (`cargo check-client`), and uploads
the client/launcher archives. Only the client is built for every platform
(owner, 29.10.2026). The servers and tools run on Linux x86_64, where the
tests build all of them. The nightly `build` matrix links the client natively
on the other four platforms, and the gateway, agent and engine server on Linux
arm64 too (`cargo build-servers`). PR and dispatched runs cannot supply release
archives.

Each package artifact includes its archive, its installers, a SHA-256 checksum
file for each and a manifest (schema 2) naming repository, commit, source CI
run, version, architecture and every file's digest. Promotion
checks the source run through GitHub's API, requires all six unexpired artifacts,
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

The six desktop package builds cache compiled dependencies. Cache keys
separate runner OS/target and compiler configuration. **Only a push to main
writes any cache**; every other run (a PR, a dispatch, the nightly, a dry run)
restores main's. A PR's own cache would live under `refs/pull/N/merge`, which
no other run can read, and on 08.10.2026 those PR writes plus the PR platform
matrix had pushed the repository past its then 10 GB, so most jobs restored
nothing (§"CI"). The limit is 25 GB since 29.10.2026.

The first debug-test partition writes a shared cache consumed read-only by
Clippy, `shipped-features` and validation. One writer avoids an
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
cargo test -p baylee-tools release
cargo test --locked -p baylee-update --test sign_script
actionlint
```

## CI

`.github/workflows/ci.yml` runs on main pushes, pull requests, nightly and by
hand. Since 08.10.2026 a change **lands on main after the local gates without
waiting for CI**: `cargo dev land` runs `cargo dev gate` and `cargo dev gate features`
under the machine's cargo lock and then pushes the gated commit
to main, only as a fast-forward, never forced. Landing is serial: under the
lock it fetches main again, merges it into the branch if it moved (a conflict
stops the run), gates that commit and pushes it, so a second lander waits and
gates on top of the first instead of being refused after its gates (`--check`
gates HEAD as it is and never merges or pushes). CI green matters at a release, which needs the main
push run of the tagged commit green (§"Cutting one").

| Trigger | Runs |
| --- | --- |
| main push (code or CI changed) | everything: fmt, clippy, test ×2, test-release, shipped-features, validate, web-feedback, deny, audit, bench, msrv, packages ×6 |
| main push (docs only) | fmt, test ×2 (tests read docs); not taggable |
| pull request | fmt, and as their inputs changed: clippy, test ×2, shipped-features, validate, deny, web-feedback |
| pull request labelled `ci:full`, or changing CI | as a main push (its packages are never promotable) |
| nightly | as a main push without packages, plus the five-platform `build` matrix (every binary on Linux x86_64, the client elsewhere, the three servers also on Linux arm64) |
| `workflow_dispatch` | `full` (default on) as a main push; `platforms` adds the matrix |

Current client releases support Windows, macOS and Linux. Android, iOS and
browser/wasm are **coming soon** (owner, 24.10.2026). Their target-specific
checks are outside CI, landing and release promotion: the Android dev-control
check and wasm job are removed, and wasm is no longer a required release job.
`cargo dev gate wasm` remains an explicit experimental command.

Development-feature builds and tests run **only locally** through
`cargo dev gate features` before landing (owner, 24.10.2026). CI and release
promotion instead require `shipped-features`: `cargo dev gate shipped-features`
checks the dependency graphs to keep `fuzz` and `mutate` out of shipped builds,
with the engine's dev graph as a positive control. It builds no development
client and reruns no gateway tests; it needs no PostgreSQL or Linux GUI headers.
The same job validates the exact dependency scope of the reviewed
`crypto-bigint` advisory exception (`docs/central-login.md` §"Bigint advisory scope").

`cargo dev ci-changes` decides what a diff touches (try it with
`git diff --name-only origin/main | cargo dev ci-changes`). Every job has a
`timeout-minutes` of about 1.5× its measured p95. The debug suite runs in two
nextest partitions (`--partition count:i/2`), each compiling for itself.
The owner excluded offline neural-training tests from both CI test profiles on
24.10.2026 (`--exclude baylee-train`). The engine, house AI and gamehost remain
covered. Trainer Rust/Python checks stay available on the training workstation;
the local full gate still includes the trainer. CI continues to compile/lint it.

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

On 29.10.2026 `test-release` twice found no cache. Its `ci-release` compile
alone took 52 and 56 minutes, and the 60-minute limit cancelled the first run
eight minutes into the tests. Its five main pushes before took 31–52 minutes.
A new stable toolchain changes the key, but the second run had the same key
as the run before it. The repository's caches sat at GitHub's 10 GB limit,
seven entries of 1.1–1.9 GB, and least-recently-used eviction dropped
`tests-optimized` whenever the package rows saved theirs. The job limit is
now 90 minutes, and the repository's cache limit is 25 GB. Space above 10 GB
is billed to the owner's account, and a budget caps it. A cold
`test-release` now means a lockfile or toolchain change, or the limit is full
again: `gh cache list` and `gh api repos/AceVik/baylee/actions/cache/usage`
show it.

The former feature job passed in 32 minutes in run `39966346270`, including
795 seconds for `dev-table-test`. Run `40019996413` spent almost seven minutes
installing Linux dependencies and hit its 35-minute limit during that same
test step. Its completed checks had passed. The development matrix is now
local-only; the release graph check above remains in CI.

The `x86_64-apple-darwin` package row (added 12.10.2026) is the arm row's
work cross-compiled on the same runner: 42–82 minutes over ten main pushes
(20.–21.10.2026), against 32–88 for the arm row. Both have 150 minutes, enough
for a cold cache (§"The macOS runner").

Three jobs (bench, msrv, features) hung on 07.10.2026 for 228, 361 and 360
minutes, all in `apt-get update` against a mirror that stopped answering; apt
is now bounded in `.github/actions/linux-deps` and every job has a timeout.

Every action is pinned to a full commit SHA with its version in a comment,
which Dependabot (`.github/dependabot.yml`) moves weekly. Since 30.10.2026
the repository enforces it (`sha_pinning_required`) and lets only GitHub's
own actions and six others run (`allowed_actions` `selected`:
`dtolnay/rust-toolchain`, `Swatinem/rust-cache`, `rui314/setup-mold`,
`taiki-e/install-action`, `EmbarkStudios/cargo-deny-action`,
`rustsec/audit-check`). A job that names any other action, or one by a tag,
fails at "Set up job"; a new action needs its pattern in
`gh api repos/AceVik/baylee/actions/permissions/selected-actions` first.
GitHub's own CodeQL workflow names its actions by tag and still runs.

### The macOS runner

The two macOS package rows can build on the owner's MacBook (M1 Max, ten
cores, 64 GB) instead of GitHub's three-core macOS image. While the
repository variable `MACOS_RUNNER` is `self-hosted`, a **push to main** (or a
dispatch, which only someone with write access can start) sends them to the
label `baylee-vm`. Every other run, a `ci:full` pull request included, and
the other macOS job (the nightly `build` row) stays on GitHub; with the
variable unset, nothing differs from before.

Measured over ten main pushes (20.–21.10.2026, minutes), GitHub's rows took
32–88 (aarch64) and 42–82 (x86_64), 27–54 of it the `dist` build with
dependencies restored from the cache. On the MacBook the same build took 19
cold and 16 with every workspace crate rebuilt (22.10.2026, with about two and
a half cores busy elsewhere; 9.6 GB at its peak). The rest of a main push then
ends with `test-release` (37–44).

Each job runs in a throwaway Tart VM (`crates/baylee-runner`):

- **Who can reach it.** A personal repository has no runner groups, so
  nothing on GitHub's side ties the runner to main's workflow: a fork's pull
  request brings its own YAML and could name the label. What keeps it off is
  the repository's fork policy, **approval required for all external
  contributors** (set 22.10.2026). Approve a fork's run only after reading it,
  and never one that changes `.github/` unread. Since 29.10.2026 two more
  checks stand behind it: the runner counts only this repository's pushes
  to main and its dispatches when it decides to start a VM, and every VM runs
  a job-started hook that fails any other job before its first step.
- **No secrets.** The package jobs hold only the read-only `github.token`, and
  release signing stays on GitHub's Ubuntu runner, so no key touches the Mac.
  The runner registers with a fine-grained token of its own (this repository;
  Administration read and write, Actions read) from the owner's login keychain
  (service `baylee-macos-runner`); the just-in-time registration it buys
  reaches the VM on stdin and is good for one job.
- **Nothing stays.** Every job gets a fresh clone of the image `baylee-runner`,
  deleted afterwards. A runner killed outright (launchd's SIGKILL after 180 s)
  cannot clean up, so `serve` first deletes stopped `baylee-vm-*` VMs and the
  label's offline registrations (29.10.2026). Softnet (`--net-softnet`) lets
  the VM reach the internet and not the home network.
- **The cargo lock.** The runner holds `with-cargo-lock.sh` while jobs are
  queued for it, so a CI build and a `cargo dev land` never share the CPU
  (under load the e2e timing tests fail): a land waits for both packages of a
  main push, and a job waits queued, which does not count against its
  timeout. Giving the lock back after each job, as first written, let a land
  in between the two (22.10.2026), and the second package waited for the
  whole land. Every `tart exec` under the lock is bounded and no job outlives
  three hours, since a wedged guest's exec never returns. Native process
  deadlines include stdin delivery and both output streams; timeout/drop
  kills the owned process group, including descendants holding pipes open.
  SIGINT/SIGTERM requests orderly VM and registration cleanup before releasing
  the lock. The LaunchAgent allows 180 seconds for shutdown. A failed GitHub
  busy-status lookup is unknown, not evidence that a running job is idle;
  the three-hour absolute limit still applies.
- **One VM at a time.** macOS allows two per Mac, and the runner can run
  both packages side by side (`Settings::max_vms`), but that buys nothing
  here: both VMs have 8 vCPUs on a Mac with ten cores, so in run 40950996798
  each dist build took 46 minutes instead of about 22, and both packages were
  done after 68 minutes against 70 one after the other. Since 30.10.2026 the
  runner starts one VM at a time (owner). With two, each VM waits for the one
  before it to reach the network or give up: two started together both stayed
  offline for twelve minutes, while the second one, started once the first
  reached github.com, had the network in two seconds (measured 29.10.2026).
- **Release trust.** The macOS archives a release promotes were built on that
  Mac. `release verify-run` checks that they come from a main push run, not
  where the run built them.

When the Mac is away, `gh variable delete MACOS_RUNNER`. A job already queued
for `baylee-vm` keeps waiting (up to 24 hours, and a push run is never
cancelled), so cancel the run and re-run it: a re-run reads the variable anew.
Back with `gh variable set MACOS_RUNNER --body self-hosted`.

Setting it up, once: `brew trust --formula openai/tools/softnet && brew install
openai/tools/tart`; Softnet's SUID bit (`sudo chown root:wheel` and `chmod
u+s` on the binary); the token in the keychain (`security
add-generic-password -s baylee-macos-runner -a AceVik -w`, which prompts for
it); then `cargo run -p baylee-runner -- prepare`, which clones
`ghcr.io/cirruslabs/macos-tahoe-base` (about 25 GB) into `baylee-runner` and
adds rustup and the pinned actions runner, and `cargo run -p baylee-runner --
install`, which copies the native binary to `~/baylee-ci` and loads the
LaunchAgent `org.baylee.macos-runner` (log:
`~/Library/Logs/baylee-macos-runner.log`).
The installer stages the executable beside its destination and replaces it
atomically, including when invoked through the already installed binary. It
creates the log directory and XML-escapes home paths in the LaunchAgent plist.
The review regression suite uses real local subprocesses for blocked stdin,
inherited output pipes, TERM-resistant children and orderly signal handling;
Tart and GitHub lifecycle calls remain mocked. Provisioning and LaunchAgent
installation need a separate live operational check.

When the pinned actions runner is a month old, `tart delete baylee-runner`,
raise the pin and `prepare` again: an older runner downloads its update at the
start of every job. Raise the base image's digest (`Settings::base`) in the
same change; it is pinned because the macOS archives a release signs are
built in it.

## Installers

The owner asked on 06.10.2026 for installers like other desktop apps have.
They stand **beside** the archives, never instead of them: the updater only
ever downloads `baylee-client-<version>-<target>.{zip,tar.gz}`, checks its
`.sig` and the signed root folder's name (§"Desktop launcher and
recovery"), and an installed client updates itself from there. So the
installers get no `.sig`; each has a `.sha256` beside it, and the publish
job writes a table of them into the release notes
(`baylee-tools release installer-sums`, which also refuses a release missing one).

| Target | Installer | Built with |
| --- | --- | --- |
| aarch64 / x86_64 macOS | `Baylee-<version>-{aarch64,x86_64}.dmg` | native `hdiutil` + Rust Finder metadata |
| x86_64 / aarch64 Windows | `Baylee-Setup-<version>-{x64,arm64}.exe` | Inno Setup 7.1.0 |
| x86_64 / aarch64 Linux | `Baylee-<version>-{x86_64,aarch64}.AppImage` | appimagetool 1.9.1, type2 runtime 20251108 |
| x86_64 / aarch64 Linux | `baylee_<version>_{amd64,arm64}.deb` | the runner's `dpkg-deb` |

In `client-packages.yml`, after `cargo tools package client`,
`cargo tools installers tools` checks the macOS system tools or fetches the
Windows/Linux tools (each download pinned by version and SHA-256), `cargo tools package installers` wraps the staged tree, and
`cargo tools release check-installers --install` installs each one on the
throwaway runner, checks it and removes it again: the dmg's app is copied to
`/Applications` and started from there until it logs `assets from
/Applications/Baylee.app/…`; the setup installs for the runner's user and
uninstalls; the `.deb` goes in with `apt`, which proves its dependencies
resolve. Without `--install` the check only mounts, extracts and lists,
which also works on a Mac. The pictures (dmg background, `.ico`, Linux icon)
are our own, drawn by `baylee-tools installers art` from the brand icon
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
  `cargo tools release check-installers` reads both executables' resource tables
  (`baylee-tools release pe-icon`) before it installs anything.
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
  app keeps the ad hoc signature `cargo tools package client` gave it; a copy dragged to
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

**Intel Macs** (owner, 12.10.2026) get their own package, not a universal
app: `x86_64-apple-darwin` is a sixth row of `client-packages.yml`,
cross-compiled on the arm `macos-latest` runner (the macOS SDK carries both
slices, so `rustup target add` is all it needs), packed by the same
`cargo tools package client` (ad hoc signature, `Baylee.app`) into
`baylee-client-<version>-x86_64-apple-darwin.zip` and
`Baylee-<version>-x86_64.dmg`. GitHub's x86_64 macOS images are retiring
(`macos-13` sat 91 minutes in `queued` after its retirement;
`macos-15-intel` is the last, until 2027), so the row does not depend on
one: `cargo tools release check-installers --install` starts the Intel app under Rosetta 2,
which the row installs if the image lacks it, and checks with `lipo` that
every program in each dmg is its target's one slice. The updater needs no
change for it: a build asks for the archive of the target it was compiled
for (`baylee_build::TARGET`), so an Intel build fetches the Intel archive and
an arm build keeps the arm one. An Intel build started under Rosetta on an
arm Mac stays on the Intel archive; nothing moves it to the arm one.

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

In `release.yml` the `signer` job builds `baylee-update-sign` with no secret
in reach, and the `sign` job downloads it with the promoted archives and runs
`baylee-update-sign release`, which signs them with the seed in the secret
`BAYLEE_UPDATE_SIGNING_KEY` and then verifies each signature against the
compiled keys. `sign` checks nothing out and compiles nothing, so no build
script runs beside the seed, and it alone enters the environment `release`,
which admits a `v*` tag and `main` (29.10.2026); the secret belongs there, not
at repository level, where every workflow on every branch could read it. The
release notes are written in `notes`, which compiles `cargo tools` without
the write token; `publish` only uploads. A secret that is not the client's
key therefore fails the release, not every player's updater. On a tag, a
missing secret fails the job and nothing is published; `notes` also refuses
an archive without its `.sig`. A dry run from `main` without the secret warns
and goes on unsigned; one dispatched from another branch is refused before
anything is built. Only the build step sets `BAYLEE_RELEASE_BUILD=1`, which is what
lets a client replace itself; a build made anywhere else only checks and
links.

`crates/baylee-update/tests/sign_script.rs` runs `release` against a test
key and holds the workflow's wiring; `cargo run -p baylee-update --bin
baylee-update-sign -- verify <archive>` checks a published archive by hand.

### The key, once

The owner does this once, on their own machine:

1. `cargo run -p xtask -- update-key` writes a new seed to
   `~/.config/baylee-release/update-signing.key` (mode `0600`, never
   overwritten) and prints the public key. The seed of 27.09.2026 is there
   already, and its public key is the one in `TRUSTED_KEYS`.
2. Store the file's content as the secret of the `release` environment:
   `gh secret set BAYLEE_UPDATE_SIGNING_KEY --env release < ~/.config/baylee-release/update-signing.key`.
   Not at repository level: there every workflow on every branch could read
   it, and a repository secret of the same name would go on being read by
   nothing once the environment has one, so a rotation written there changes
   nothing.
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

`cargo tools package client` requires the Bevy client, `baylee-launch` and
`baylee-seat` in its input directory. The main CI package workflow builds all
three with the dist profile (`cargo build-client`).
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

"Restart to update" (`baylee_update::relaunch`, `docs/client.md` §"Restarting
into an update") needs no launcher change: the runtime leaves its own
executable behind as a helper that waits for the old runtime (the end of
its stdin) and the old launcher (the lifetime lock) before it starts the
package's launcher, which an older launcher would otherwise refuse as a
second instance; the launcher hands its stdin to the runtime, as every
shipped one does.

**Links from the website** (`docs/client.md` §"Links from the website")
changed the launcher once, in the 30.10.2026 packages: a second launcher
whose arguments hold a `baylee:` link no longer refuses at the lifetime
lock (or the entry lock of a launcher still starting) but hands the link
to the running client's door (`baylee_update::link`) and ends. The door's
frame is frozen like the rest of the launcher's protocol. Every package
also claims the scheme (the bundle's `CFBundleURLTypes`, the setup's
`HKCU\Software\Classes\baylee`, the desktop entry's
`MimeType=x-scheme-handler/baylee;`), and `check-installers` checks each.
Neither arrives through an update: a player gets links by installing a
new package by hand once. On macOS a downloaded generation is a whole
`Baylee.app` with the same `Info.plist`; LaunchServices registers it when
it runs. A spike on 30.10.2026 found the original bundle preferred when
both claim the scheme; an original from before that date claims none, so
until it is replaced a link may start the generation's launcher, which
then keeps a state directory of its own.

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

## Server deploys

The server deploys itself: a timer runs `baylee-deploy watch` every minute
(crate baylee-ops, run by `scripts/server/baylee-deploy.service` and `.timer`), stages the newest
`v*` tag the first time it sees one, swaps agent and engine at once and the
gateway once no game of this machine is running;
`BAYLEE_DEPLOY_HOST=user@host cargo dev deploy [--force]` does the same for a
commit by hand (the repository names no server; the host is the operator's).
`crates/baylee-ops/src/deploy.rs` describes the two halves. A server still
running the shell script from before 20.10.2026 takes its next deploy by hand
(`docs/dev-tools.md` §"Tests").

An operator can run local programs at three fixed points of that deploy
(`prepare`, `before-switch`, `after-switch`) to keep another service on the
same machine in lockstep, and can require release tags to be signed by a key
of their choosing: `docs/deploy-hooks.md` is the contract, including the
admission hold (`BAYLEE_ADMISSION_HOLD`) a deploy with hooks drains every
agent's games under, and the dispatcher's exit codes.

### What the server builds with

The server builds each release itself, as the deploy user, in its checkout
(`baylee-deploy stage`). It needs:

- `rustup`'s `stable` at least the workspace's `rust-version` (`Cargo.toml`).
  A release that raises it needs `rustup update stable` there first; the
  deploy otherwise stops at "is not supported by the following packages"
  (22.10.2026).
- A C toolchain, `pkg-config` and the headers of `libsecret-1` (Debian and
  Ubuntu: `libsecret-1-dev`, which brings GLib's). `baylee-keyring` links
  libsecret on Linux, and the engine server, the seat bridge and the seat
  agent reach it through `baylee-client-core`. CI installs the same package
  in `.github/actions/linux-deps`, beside the client's audio, udev and
  Wayland headers, which the server does not need.
- For the experimental browser client at `/play` (coming soon): `trunk` (0.21.14) and the
  `wasm32-unknown-unknown` target. Without them the installed client stays.
- For the feedback service's UI: `npm`. Without it the installed UI stays.

A build that fails shows its first error, and for the first two causes
one sentence on what to do.
