# Tower Echoes operator commands

The separate `baylee-tower-service` executable owns Tower publication, economy
and execution authority in the Baylee repository. Central owns identity and
permanent accomplishment receipts. This runbook describes local/offline
provisioning; release approval, calibration, retention and the final least-privilege
runtime grants remain separate release requirements.

The Commander run catalog uses generator 4 / native `tower-5` and API header
`baylee-tower-version: 6` on the existing `/v2/player` endpoint and host routes.
Native players may also use `/v2/player/ws/6`; reverse proxies must forward
WebSocket upgrades and allow the 15-second ping/pong heartbeat. Migration 015 adds
transactional invalidation triggers with invoker rights and a fixed `pg_catalog`
search path. Each authority listens on its schema name; hints are private-data-free
and snapshots always recheck account ownership. No new database role, external
broker, credential or production network endpoint is required.
Older API clients receive 426 before authentication rather than undecodable new
families. Deploy matching service, gateway and client builds together. Retain the
exact `tower-2` / `tower-house-19` executors for generation-one runs and
`tower-3` / `tower-house-20` for generation two and `tower-4` /
`tower-house-21` for generation three. Publishing a new cohort preserves
existing runs and their snapshots. The current executor uses `tower-5` /
`tower-house-22` and refuses retained manifests. All prior catalog/offer streams
remain frozen; new families cannot enter old runs. An old run cannot silently
resume on the current binary, even if its source registry appears compatible.

API 5 introduced the generation-four reward phase, six-slot run inventory and
39+1 Commander formations. Generation four is the new-run default; enrollment
still requires an explicit immutable publication and matching installed executor.
The separate `baylee-tower-content` adapter builds five reviewed-card starter
collections and validates their native metadata; it does not certify balance.
After verified settlement, claim
the pending cards before shopping or abandoning. Reconnecting preserves the
same candidates. An already-owned singleton choice grants one gold instead.

API 6 adds cohort-scoped Tower rankings and a separate reversible publication
setting. Migration 014 stores one best completed verified run per account/cohort/
opponent lane. Every all-echo run belongs to the echo board; any verified house
opponent puts the entire run in the house board. Sort by victories descending,
then defeats/rules draws ascending; true ties share competition rank. Time, gold,
life, and stable row IDs never break a score tie. The response contains the top 25
and the authenticated owner's own best. Abandoned/partial/technical runs do not
count. A final pending card reward does not delay the already verified score.

Public listing defaults off, independent of echo consent. The service obtains
display name/tag from live Central introspection, never from request fields.
Names refresh when the owner reads a board; removing the public identity withdraws
the listing. Publication and withdrawal use a monotonic compare-and-set revision;
an old publish retry cannot undo a later withdrawal. Deleting the Tower player
cascades its bests. This does not implement cross-service Central account-erasure
delivery: operators must include the Tower player in their deletion workflow and
must not claim that Central deletion alone has removed Tower data. Private bests
survive raw-run retention until player/cohort removal; the new projection starts
with verified completions after migration, without inventing scores for old runs.
Apply migration with the schema owner and renew the reviewed runtime table grants
before starting the matched API-6 service. Production retention/privacy approval
remains a release requirement.

## Input and authority

An owning Rust launcher supplies one JSON value through private stdin and closes
the pipe. Never place database URLs or credentials in files, command arguments,
logs or shell history. Reuse the reviewed `central_api::Token::generate` for
independent credentials; do not derive tokens from UUIDs or names. Retain them in
the operator's approved secret source. Nothing prints or recovers credentials.

`--admin` executes one operation and closes its pool. Its input is:

```text
{
  "configuration": {"database_url": <private URL>, "schema": <schema>,
                    "development": <explicit boolean>},
  "operation": {"action": <closed action>, ...fields below...}
}
```

The whole input is bounded to 196 KiB, unknown fields are errors, and `service`
must be omitted/null. The database TLS, schema and explicit numeric-loopback
development rules are exactly the [service configuration rules](dev/environment.md).
No listener, verifier child or courier starts in this mode. An exact current
migration history and effective membership in the SQL schema-owner role are
required before any operation. A Host, Publisher or other application credential
does not supply that SQL ownership. Do not give runtime roles membership in the
owner role. Database superusers already have ownership-level authority.

`--check-admin` accepts the same input, validates its shape and local bounds, and
exits without opening the database. It does **not** establish operator permissions,
existing IDs, immutable publication compatibility, card coverage or game balance.
Both modes reject extra arguments. Success exits zero; refusal exits nonzero with
a fixed diagnostic containing no input. A failed preflight never changes state.

## Closed operations

| `action` | Additional fields | Result and semantics |
| --- | --- | --- |
| `provision` | `name`, `role`, `credential` | `{"result":"operator","id":<UUIDv7>}`. Role is exactly `host`, `verifier`, `supervisor`, `publisher` or `courier`; credential is 64 lowercase hex digits. Names are 1–160 bytes, trimmed and without control characters. An exact enabled name/role/token retry returns the same ID. Changed name/role or a revoked credential refuses. |
| `revoke` | `operator` | `{"result":"done"}`. Disables an existing operator; exact repeats succeed. Live authenticated principals lose their role on the next protected operation. No history is deleted or replacement credential minted. |
| `approve_host` | `host`, `origin` | `done`. Binds an enabled Host to one canonical approved gateway origin. Production requires HTTPS; development allows only numeric-loopback HTTP. An exact repeat succeeds; changing the origin refuses. |
| `publish` | `publication` | `{"result":"published","cohort":<UUIDv7>}`. Accepts the typed `baylee_tower_store::Publication`: frozen cohort/executor identity, reviewed catalog/prices and 5–8 distinct Commander starters (three for retained generations). Exact canonical repeats return the same ID. Different content under the same contract refuses. |
| `publish_house` | `publisher`, `cohort`, `stage`, `archetype`, `build` | `done`. Additionally requires a live dedicated Publisher credential. Stage is 1–12, archetype 1–8 (1–3 for retained cohorts). The typed `Build` contains `formation`, `deck`, `active`, `potions`; it must validate against that cohort's saved catalog and loadout bounds. Run equipment has six slots; retained builds keep three. An exact repeat succeeds; changing a published fallback refuses. |
| `pause_enrollment` | `cohort` | `done`. Closes new enrollment, preserving existing runs, receipts and immutable content. Exact repeats succeed. Republishing the same content cannot reopen it. |

All IDs must be UUIDv7. There is no arbitrary SQL, script, state patch, currency
grant, award injection, unrevocation, content rewrite or reopening operation.
No output includes a database URL, credential or private house build. The binary
returns only the closed results above. A lost response is resolved by retrying the
exact command; never substitute a fresh credential merely because a reply was lost.

## Bring up a reviewed cohort

1. Migrate the dedicated PostgreSQL 18+ schema with separate owner credentials
   using `--migrate`. Normal startup refuses missing/different migration history.
2. Prepare and review the complete immutable publication. Play/calibrate all
   starters and fallbacks with its exact retained Tower executor before enabling
   public enrollment. Shape validation is not a playability certificate.
3. Provision separate Host, Verifier, Supervisor, Publisher and Courier credentials.
   No role implies another. Use `approve_host` for each reviewed gateway origin;
   advertisements alone cannot select an arbitrary client destination.
4. Publish the cohort and its reviewed, explicitly labelled house fallbacks. Keep
   the returned cohort ID and exact executor available for outstanding runs.
5. Independently provision Central's scoped award issuer for the same Tower origin.
   An ordinary gateway/challenge bridge cannot issue these receipts. Never reuse
   that issuer credential for the local Courier role.
6. Start the service with its runtime SQL credentials, configured cohort verifier/
   supervisor workers and award courier. Supply gateway Host credentials separately.
   Readiness is not proof of complete cohort coverage or Central availability.

For incident containment, pause enrollment and/or revoke the compromised operator.
Already committed results and outbox records remain durable. Revocation is a
live authorization boundary, not retroactive cancellation of transactions that
committed before it. Replacing a revoked Host requires a fresh credential/ID and
explicit handling of retained assignments; there is no silent origin takeover.

## Reproducible focused checks

With an owned synthetic PostgreSQL fixture, run the service's `admin` integration
target and binary tests, then the existing `bootstrap` and store `operators`
targets. The admin fixture exercises real child stdin/stdout, owner and restricted
SQL roles, every command, retry/immutability, live revocation and fixed diagnostics.
It bounds child output/deadlines, reaps commands and removes its owned schemas and
login role. Its all-basic-land publication is deliberately structural test data,
not a balanced release deck or approved content package.
