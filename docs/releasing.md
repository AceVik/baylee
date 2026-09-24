# Releasing

The PM owns releases and the workspace version (`docs/dictionary.md`, Revier).

## What a release is

A tag `v<version>` on a commit that `ci` passed on makes
`.github/workflows/release.yml` publish a GitHub Release. The release has one
archive per desktop target (Linux, Windows x86_64/aarch64 and macOS aarch64),
each with a `.sha256` beside it. Every archive holds the client, `assets/`
(the fonts load from there at run time, together with their licences),
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
