# Native host handover and obsolete-controller retirement

All paths below are relative to the sealed package:

`/home/ace/baylee-data/v5/ops-codex-20261028/refactor-investigations-20261029/native-host-deployment-20261029-01a187a6-9180-7739-a78a-7ecbd0758c01`

The owner performed the live handovers after fresh checks. Preview on 28766 and
lane2 on 28777 moved first; the verifier confirmed installed unit hashes,
immutable native executable identities, database/agent health and unchanged
preview model pins. Arena on 28776 remained untouched until its old game was
shown to be an abandoned owned collector session.

The agent log directly binds engine PID 278521 to game
`01a182df-c0e5-77cf-b429-9ffbb11c283c`, starting on October 28 at 14:17:49 UTC.
Its ownership chain was Python controller 95824 → agent 95857 → engine 278521.
Only that agent and engine had TCP connections to the arena; no arena seat or
collector remained. The collector was idle after an HTTP503 error, its STOP
file dated October 28 15:34 CET, and the unfinished attempt had no directory.
The preceding attempt receipt identifies the preceding completed game.

Read-only database metadata confirmed the current record was incomplete with
13 chunks and 18,566 bytes. An older incomplete record
`01a18269-ca01-71ea-a842-5d222e0843a2` contained 7,521 bytes. The owner then
stopped only the dedicated arena host, checked its owned processes were gone,
installed the sealed arena unit and started its native replacement. No winner
or outcome was manufactured to make either record complete.

Final verification found:

| Host | Native controller PID | Health |
| --- | --- | --- |
| Preview 28766 | 1586446 | database true, one agent, zero running games |
| Lane2 28777 | 1586447 | database true, one agent, zero running games |
| Arena 28776 | 1592375 | database true, one agent, zero running games |

All three installed units match their staged hashes and execute the sealed
`bin/serve_local4`. All three actual release `--check` validations passed.
The known old controller/arena PIDs 9331, 95768, 95824, 95828, 95857 and 278521
have no live identity. Both incomplete database rows still have `complete=false`,
null end time, null winners and null end reason, with unchanged byte counts.
They remain outside the complete-only exporter and Header-to-End admission;
verification did not export or admit them.

Under separate explicit owner authorization, the retirement helper checked all
five obsolete units were inactive/failed, MainPID zero and trigger-free, backed
up every original file durably, removed those exact configuration files and
masked those exact unit names:

- `baylee-arena-v5.service`
- `baylee-arena-v5-lane2.service`
- `baylee-continuous-v5-training.service`
- `baylee-v5-full-coverage-eval.service`
- `baylee-terra-flash-balanced.service`

Every resulting unit is masked to `/dev/null` with MainPID zero. The balanced
unit retains its failed state as history. No active host unit was included;
retirement invoked neither stop nor start. Original experiment sources,
receipts and artifacts remain unchanged.

Durable receipts:

- `post-migration-observation-01a187b2-64a7-7681-9bea-7b9201173f5e.json`:
  initial native-host checks, orphan ownership and bounded read-only DB evidence.
- `obsolete-controller-retirement-01a187b3-2938-73f4-b5f6-3dc79b6fecf5.json`:
  original hashes, backup paths, authorization scope and actual masked states.
- `unit-backups/historical/retired-01a187b3-2938-73f4-b5f6-3dc79b6fecf5/`:
  read-only originals and durable retirement intent.
- `final-host-migration-verification-01a187b5-1a4d-73fe-97c3-20a77cc8a700.json`:
  all three native identities, installed-unit/profile checks, health, dead old
  identities, preserved incomplete records and all five masks.

The verification helpers are read-only; `retire_inactive.rs` contains only the
five-name retirement operation explicitly authorized for this handover. No
token or process environment was read. These helpers started no additional
training or collection job.
