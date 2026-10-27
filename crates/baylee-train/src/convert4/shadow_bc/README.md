# Verified shadow BC conversion

Use a NEW output directory:

```
convert4 --runs /absolute/raw-shadow-run --out /new/bc-copy \
  --include-passive-obs --shadow-bc --threads 4
```

Every selected completed game requires its completed `shadow/<source-game>.jsonl`.
No source record, executed action, rollout probability, or original dataset is
rewritten. Default conversion is unchanged. All original engine hashes are
replayed before feature frames are published. Labels additionally verify run
bytes, source game key, checkpoint/model/encoder7/catalog identities, record
SHA256, byte offset/input line number/seat/executed action/hash, decision clock,
authorized view, actual chronological seat knowledge, encoded state and exact
candidate ordering. The view-only teacher is rerun with its original independent
seed and public teams. Targets are reconstructed from its action, then checked
for repair-free decode/re-encode; stored raw targets are never trusted.

Labelled decision targets become House/no-logprob, preserving the executed
thinking depth. Every unlabelled decision has policy loss weight zero and no
behavior probability. Passive K0 observations, chronological cursors, outcome
and auxiliary targets remain intact. Missing, duplicate, unknown, repaired or
mismatched labels fail closed. Mapping failures originally counted by the
collector legitimately leave unlabelled decisions.

The BST4 binary layout is unchanged. Optional per-frame JSON provenance is
retained separately in `shadow-bc/game-NNNNNN.jsonl`, keyed by converted seat and
cursor and linking the source run hash/game/input plus teacher and executed
behavior. Final metadata binds these files by SHA256. Source identity is
`(SHA256(run.json), source_game, SHA256(record))`, never the local numeric converted game ID.
Metadata additionally retains original absolute run/label/record-shard paths,
hashes and record member offsets/lengths.

Admission contract:

- `shadow_bc_version: 1`
- `shadow_bc_contract: "verified_view_only_teacher_v1"`
- `shadow_bc_status: "complete"`
- `ppo_eligible: false`
- `policy_provenance_version: null`
- `shadow_bc_sources`: immutable source/game and converted-label bindings
- `shadow_bc_label_sha256`: companion-label file hashes

`dataset_header.json` is marked `in_progress` during conversion. Do not train
from streaming ready files before the whole conversion completes. Any rejected
game prevents final `meta.json` publication. Downstream packaging must retain
this BC contract and companion provenance/hashes; ordinary binary caches alone
do not retain the optional JSON field. PPO must explicitly refuse this dataset.
No change to model inputs, action ABI or sampling version is introduced.
