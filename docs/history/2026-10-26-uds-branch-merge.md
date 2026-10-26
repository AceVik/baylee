# Reviewed `agy/uds-2` merge — 26 October 2026

Merged `agy/uds-2` at `d3eb1aac3` into prior main `4794d0240` without
conflicts, producing merge commit `6191e4ed0`. The 36 incoming Claude commits
received independent review: Sol reviewed cards/readers and Astra reviewed
engine changes. All 54 source commits carrying `Review: pending` have
independent review closure, including the integration fixes. This merge does not claim completion
of any set.

Review corrections preserve Insubordination's captured player, make Phyrexian
Negator's sacrifices simultaneous, and count Multani's Decree's successful
destructions even when their destination is replaced, with one aggregate
life-gain event. Generic fixes cover nested/APNAP payment context, source-version
protection and destruction sweeps, and adjacent equal-amount untargeted
creature/player damage. Canonical tests retain literal subject identities and
exercise combat restrictions through actual legal offers and outcomes.

The focused run passed **35/35** (`/tmp/baylee-uds-focused-2.log`). Initial
compilation raced creation of a test module; two later test compilation errors
were corrected before that passing run.

- Deliberate RED mutations: **8/8 detected; every mutated source restored exactly** (`/tmp/baylee-uds-red-proof/results.json`).
- Restored focused GREEN: **35/35 passed** (`/tmp/baylee-uds-restored-focused.log`).
- Rules gate: **21,355/21,355 passed, 17 skipped, exit 0**
  (`/tmp/baylee-uds-merge-rules.log`); formatting and lint steps passed.
- Full desktop gate: **23,606/23,606 passed, 25 skipped, exit 0**
  (`/tmp/baylee-uds-merge-full.log`); formatting, Clippy, validation, API and
  all four API tests passed.
- Feature gate: **passed, exit 0** (`/tmp/baylee-uds-merge-features.log`).
  Client: **2,290 passed, including 39 devctl tests**. Client/dev-table Clippy,
  dev-table tests, dev-dylink, test-support, train without play, fuzz, mutate,
  verification-hooks-shipped and crypto-bigint-scope all passed.

Both final gates ran on `2dca3227cc796b87db78ec74d112019c4e7cefe7`.

The first feature gate failed only its dev-table-test step after the shared
`baylee-postgres-1` container stopped at `2026-10-26T17:52:16Z` (Docker exit 0).
Its log remains at `/tmp/baylee-uds-merge-features-initial.log`. A separate
`e2e_social_reports` run reproduced five `PoolTimedOut` failures while connecting
to the database. After restarting the existing container and confirming
`pg_isready`, the unchanged test binary passed **5/5 in 5.03s**
(`/tmp/baylee-uds-social-reports-restored.log`). No source or concurrency changes
were made. All other initial feature steps passed, including **2,290 client
tests, 39 of them devctl**. The complete feature rerun then passed as recorded above.

Table generation ran twice, followed by `--tables --check`: all exited 0
without changing files. Validation changed from 10,693 cards / 10,352 finished
(2,578 hand-owned, 7,774 machine-owned) / 341 stubs to **11,083 cards /
10,600 finished (2,671 hand-owned, 7,929 machine-owned, 0 untested) /
483 stubs**. These are registry totals, not completed-set counts.

The card-test hook captured real Write events for 399 changed cards and a
nonempty Stop event. It exited 0 with empty stderr and `debt_cleared: true`
(`/tmp/baylee-uds-card-test-hook.json`).

The final lint-only fixes (`6e2fc3ff8`, `04793cca5`, `210157e93`,
`a524065d1`) were independently reviewed.

The deliberate RED results measure regression sensitivity; they are separate
from the real integration failures above.
