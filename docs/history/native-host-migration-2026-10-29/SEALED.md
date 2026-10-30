# Release-sealed host package — staging only

Stage directory:

`/home/ace/baylee-data/v5/ops-codex-20261028/refactor-investigations-20261029/native-host-deployment-20261029-01a187a6-9180-7739-a78a-7ecbd0758c01`

The two binaries were built at **73a2fbcc5696941d53deafda4eab1adaf567b2a1**.
This identity comes from the owner's explicit successful build confirmation,
not the repository HEAD when staging ran. Subsequent changes under the
operations sources were confined to batch-audit code and legacy-column tests;
the two host binaries' behavior was unchanged.

| Immutable staged binary | SHA256 |
| --- | --- |
| `bin/serve_local4` | `fd8d83880203d12a3116dc11ebb9205df5a391b08f400505aa04f1e524986391` |
| `bin/collect_local4` | `4e7c4dba3c3fa0770944e882fd25b7d026bd437890e433c6d72d8398553fe732` |

`release-source-receipt.json` records the build identity.
`reviewed-release-bins.json` records literal release sources, byte counts,
timestamps, hashes and that source receipt. `immutable-binaries.json` binds the
review hash, staged binary hashes and final runtime manifest hashes.
`verification-receipt.json` records the exact validation argv, successful exit
codes and unchanged live unit identities. The receipts and runtime manifests
are read-only; service binaries are mode 0555.

Commands already completed, with paths interpreted relative to this stage:

```text
prepare-tool seal STAGE STAGE/reviewed-release-bins.json
bin/serve_local4 --manifest STAGE/manifests/preview.json --check
bin/serve_local4 --manifest STAGE/manifests/arena.json --check
bin/serve_local4 --manifest STAGE/manifests/arena-lane2.json --check
```

The seal operation also runs full package verification. Each actual release
profile check returned `Pinned local profile verified; nothing started`.

Files prepared for the owner's later installation:

| Staged unit source | Existing destination | Runtime manifest |
| --- | --- | --- |
| `units/baylee-v5-live.service` | `/home/ace/.config/systemd/user/baylee-v5-live.service` | `manifests/preview.json` |
| `units/baylee-arena-v5-gateway.service` | `/home/ace/.config/systemd/user/baylee-arena-v5-gateway.service` | `manifests/arena.json` |
| `units/baylee-arena-v5-gateway-lane2.service` | `/home/ace/.config/systemd/user/baylee-arena-v5-gateway-lane2.service` | `manifests/arena-lane2.json` |

All staged ExecStart paths use the immutable stage directory. Existing gateway,
agent, engine, seat and model paths and hashes remain unchanged. Original unit
backups are in `unit-backups/`, with their hashes and original state in
`deployment.json` and `prepared-artifacts.json`.

No unit was installed, stopped, started, reloaded, disabled or masked. Preview
PID 9331, arena PID 95824 and lane2 PID 95768 remained active throughout. The
arena's active-game hold remains mandatory; preview and lane2 require the
owner's fresh idle and health checks before any live handover. No credentials
or process environments were queried.
