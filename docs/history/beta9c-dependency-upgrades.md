# Dependency migration for 0.1.0-beta.9c

The owner requested clean Cargo builds, dependency upgrades including major
versions, migration, green gates and merging to main on 2 November 2026. The
exact version is `0.1.0-beta.9c`, deliberately below the previous
`0.1.1-beta.9b`; the updater retains its SemVer downgrade protection. No release
tag or production deployment is part of this preparation.

Both primary Cargo target directories were cleaned after preserving non-build
QA artifacts under each repository's `.git/clean-preserved-20261102`. Changes
were prepared in isolated worktrees because the original Central checkout held
uncommitted work and index conflicts. No such work was reset or overwritten.

## Runtime and tool upgrades

`cargo upgrade --incompatible allow --pinned allow` and `cargo update` cover
the main workspace and the independent API-documentation workspace. The latter
pins **`oas-forge = "=0.2.1"`**, including its own lockfile. Generated OpenAPI
and AsyncAPI documents carry the new application version. Its four route tests
and independent advisory scan pass.

Direct changes include parking_lot 0.13, ICU casemap 2.4, sysinfo 0.40.1 and
libloading 0.9. The locked Bevy graph advances to 0.20.1, and the remaining
compatible transitive dependencies are refreshed. Bevy now admits ruzstd
0.9.1, so the RUSTSEC-2026-0344 exception is removed from deny and CI audit
configuration. The existing ttf-parser maintenance notice and narrowly bounded
Central bigint exception remain documented; no additional ignore was added.

The final native resolution also picks up ctutils 0.4.8, published during this
work. Its two changed source files were compared with 0.4.7: additional
constant-time barriers apply to Thumb-1 and RISCV32 nonzero comparisons, and
the manifest's dependencies/features remain unchanged. It forbids first-party
unsafe; the already existing cmov implementation remains its dependency.
Central's own lockfile retains the owner's exact accepted 45-change scope.

Two newer direct API generations cannot be used independently of current
Bevy: it exposes glam 0.33.2 and accesskit 0.24.1 types. Both are pinned to those
compatible versions; adding glam 0.34 or accesskit 0.25 in parallel produces
incompatible public types. Bevy's own accessibility documentation requires
matching its AccessKit version. The AccessKit 0.25.2 password disclosure fix
was reviewed against our input boundary: masked fields supply bullets to Bevy,
not the password string (`lobby/field.rs`, `spawn_run`). Existing masking and
explicit reveal/re-mask tests cover that boundary.

The final dry-run reports the remaining 94 direct registry requirements at
their latest versions, excluding these two deliberate Bevy constraints; the
eight Central Git dependencies are upgraded together to the reviewed Central
merge. OPAQUE's required older digest/RNG generation is documented in Central.

## Migration behavior and validation

The monitor handles sysinfo's fallible initialization, native interface names
and renamed CPU-usage API. Unavailable uptime/load values are null on the wire
and visibly unavailable in the bundled UI. Physical rates use native names for
filesystem lookup and avoid an intermediate interface allocation. A headless
browser check fed synthetic WebSocket samples for missing, zero and restored
metrics; all rendered correctly without JavaScript errors, and the browser was
closed afterward.

The native resolver's new ureq timeout fields preserve the existing absolute
DNS/connection/TLS deadline; read/write budgets remain enforced by the owned
stream. Existing stalled/trickling-peer transport regressions remain applicable.
The updater gains a regression for the deliberately lower base version.

The full gate exposed an exact-pose failure on a zero-duration animation frame:
quaternion interpolation rounded the identity rotation even at `t = 0`. The
glide now keeps such a card active without writing its transform; explicit
hidden/reduced-motion/placement snaps still take precedence. An independent
regression covers both sparse and dense active sets, unchanged component ticks,
resumption and hidden snaps. The differential reference's advancing-frame
arithmetic and all exact snapping assertions remain intact.

Initial complete-workspace Clippy and both dependency scanners passed with the
existing reviewed exceptions. Full gates and feature checks use an owned,
randomly credentialed PostgreSQL 18 container bound only to loopback, with
verified cleanup. Final gate and integration evidence is recorded with the PR.

The first full run executed 24,911 tests: 24,897 passed, the zero-time pose
test failed before its fix, and 13 gateway tests failed after the fixture's
1 GiB tmpfs filled and PostgreSQL stopped. The fixture was removed and its
listeners verified closed. The final run uses 4 GiB of bounded temporary
storage, a 6 GiB container memory ceiling, `max_wal_size = 128MB`,
`min_wal_size = 32MB` and a 30-second checkpoint timeout. These are local QA
fixture settings; no production database configuration or test assertion is
weakened. All 13 motion regressions, including the new zero-time test, pass.
The 37-test rerun covering those motion tests, ticketed sockets, social
presence/table isolation and the database-backed language/name caches passes
against the corrected fixture, which was removed with its port verified closed.

The real PostgreSQL 18.6 Central/Gateway integration passes with the updated
SDK: legacy cutover/sign-in, restore, admission and personal transport are
exercised. This coordinated test used the documented local path override;
Central PR #61's rebase merge has a byte-identical source tree. All eight final
SDK pins use that immutable merged revision, `df00faf8d9f434cfa2bf030b99e11edd04b08e25`,
and the local patch configuration was removed before landing. The final
958-package native lockfile passes cargo-audit 0.22.2 with the two existing
reviewed exceptions and cargo-deny 0.20.2. Final gates use the Git revision.

The final `cargo dev gate` passes formatting, complete-workspace Clippy,
all 24,912 tests (39 existing skips), validation against all 12,542 cached
Scryfall payloads, the generated API comparison and four API-documentation
tests. The test suite took 3,059 seconds under the observed host load; this
is correctness evidence, not a performance benchmark. Its temporary database
used 195 MiB of the bounded 4 GiB storage after the complete suite.

The client development-feature variant also passes Clippy and 2,392 tests,
including 42 `devctl::` tests against the gate's minimum of 15. These features
remain local-only; the shipping graph checks keep them out of release builds.
The complete `cargo dev gate features` and separate `cargo dev gate
shipped-features` both pass: gateway `dev-table` Clippy/tests, test support,
training without `play`, fuzzing, simulation, mutation, feature isolation and
the reviewed bigint source/graph scope. The Linux-only collector check is
reported as inapplicable on this macOS host, as the existing gate specifies.
The owned PostgreSQL container was removed, its port closure verified by the
fixture, and no test or gateway process remained from the worktree.
