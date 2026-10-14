# Redaction and reproduction notes

These are sanitized archival exports of one completed measurement session.
They are evidence of that session, not a drop-in benchmark runner.

- Personal home prefixes are replaced by `/Users/benchmark-user` in logs and
  recorded scripts. This is a neutral placeholder, not an actual operator.
- The artifact root is normalized to `/tmp/baylee-sba-benchmark`; other
  temporary paths use `/tmp`. The session label is `engine-sba-reset`.
- The sample reports' Mach-O binary-image UUIDs are replaced by
  `IMAGE_UUID_REDACTED`. Those are image identifiers, not host UUIDs; no
  recognizable host UUID, hostname, credential or secret was found in the
  exported files. Generic hardware, OS/toolchain versions, process/thread
  IDs, sample counts, symbols, addresses and source locations are retained.
- Original executable and pool SHA-256 values in `provenance.json` are
  unchanged. `measurements.json`, `summary.json` and `profile-summary.json`
  are byte-identical to the pre-redaction exports. No sample or timing value
  was edited. `files.sha256.json` describes the sanitized public files and
  excludes itself.

Unredacted recordings and the sealed binaries, dSYMs and pool dumps remain
in the operator's private temporary storage and are not part of the repo.

The scripts record the original sequencing and commands, with the path/label
substitutions above. They assume a particular baseline checkout, local lock
wrapper, prepared pool dump and fresh output directory, and the driver
intentionally edits the SBA guard between A and B. Do not execute them as
provided against the current checkout. For a reproduction, inspect them and
supply a fresh isolated baseline checkout, appropriate local paths and one
outer build/benchmark lock. The committed guard and tested Rust sources were
not modified by this redaction.
