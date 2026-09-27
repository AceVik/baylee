# Releasing

The PM owns releases and the workspace version (`docs/dictionary.md`, Revier).

## What a release is

A tag `v<version>` on a commit that `ci` passed on makes
`.github/workflows/release.yml` publish a GitHub Release. The release has one
archive per desktop target (Linux, Windows x86_64/aarch64 and macOS aarch64),
each with a `.sha256` and a signature (`.sig`, §"Signing") beside it. Every
archive holds the client, `assets/` (the fonts load from there at run time,
together with their licences),
`LICENSE`, `NOTICE` and a `README.txt`. `scripts/package-client.sh` builds the
archive and also runs locally.

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
2. Wait for `ci` to go green on that commit. The workflow refuses a tag on a
   commit without a passing `ci`, and a tag that differs from the version.
3. `git tag -a v0.2.0 -m "Baylee 0.2.0" && git push origin v0.2.0`.

To dry-run, start `release` by hand (`gh workflow run release.yml`). It does
the same builds and uploads them as workflow artifacts, versioned
`<version>-dev.<commit>`, and publishes nothing.

## Signing

The client installs updates by itself (#326, `docs/client.md` §"Updating"),
and only an archive whose Ed25519 signature verifies against a key compiled
into it (`crates/baylee-update/src/sign.rs`, `TRUSTED_KEYS`). So every
archive gets a `<archive>.sig` (base64 of the 64-byte signature) before it is
published. The `.sha256` beside it only catches a broken download; anyone who
can replace an archive can replace its checksum.

In `release.yml` the `sign` job downloads every build's archive and runs
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
