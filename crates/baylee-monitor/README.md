# Baylee Pulse

A small read-only Linux workstation monitor. One shared sampler, direct NVML
telemetry, embedded responsive UI, live WebSocket snapshots. No database,
JavaScript build, external assets or GPU inference. See
`docs/workstation-monitor.md` in the repository for operation and metric semantics.

Build: `cargo build --profile tools -p baylee-monitor -j 2`.
Run: `target/tools/baylee-monitor --bind 127.0.0.1:8787`.
For a trusted LAN, use the machine's explicit LAN IP instead of loopback.

The source download is a JSON object mapping file names to source text. It
contains the exact files embedded in this binary, the workspace manifest,
dependency lockfile and AGPL license. To reconstruct the workspace from that
download (using Python's standard library, not needed to run the service):

```python
import json
from pathlib import Path
files = json.loads(Path('baylee-pulse-source.json').read_text())
root = Path('pulse-source')
for name, text in files.items():
    if name == 'workspace-Cargo.toml':
        path = root / 'Cargo.toml'
        text = text.replace('members = ["crates/*", "xtask"]',
                            'members = ["crates/baylee-monitor"]')
    elif name in ('Cargo.lock', 'LICENSE'):
        path = root / name
    else:
        path = root / 'crates/baylee-monitor' / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
```

Build inside `pulse-source` with the command above. Cargo may prune unused
workspace dependencies from the lockfile. The service never reads source files
or arbitrary paths at runtime. Copyright Viktor Machnik; AGPL-3.0-only.
