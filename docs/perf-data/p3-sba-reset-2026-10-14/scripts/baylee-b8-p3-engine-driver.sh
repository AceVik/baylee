#!/bin/bash
set -euo pipefail
cd /Users/benchmark-user/.codex/worktrees/b8-perf-cleanup/baylee
test ! -e /tmp/baylee-sba-benchmark
source /tmp/baylee-b8-p3-common.sh
phase=preflight
record_status() {
    local driver_exit=$?
    printf "phase=%s exit=%s\n" "$phase" "$driver_exit" > "$OUT/driver-status.txt"
}
trap record_status EXIT
printf '%s\n' "$BAYLEE_SESSION" > "$OUT/session.txt"
cp /Users/benchmark-user/.baylee-locks/cargo/who "$OUT/lock-owner.txt"
cp /tmp/baylee-b8-p3-{common,build-A,build-B,measure,profile}.sh "$OUT/"
cp /tmp/baylee-sba-benchmark-driver.sh "$OUT/"
cp crates/baylee-engine/src/sba/tests/cow_reset.rs "$OUT/cow_reset.rs"
shasum -a 256 /tmp/baylee-b8-pool-before.json > "$OUT/original-pool.sha256"
python3 - <<'PY'
from pathlib import Path
import hashlib, json
root=Path('crates/baylee-engine')
paths=sorted(p for p in root.rglob('*') if p.is_file())
data={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
Path('/tmp/baylee-sba-benchmark/engine-sources-A.json').write_text(json.dumps(data,indent=2)+'\n')
PY
phase=red-and-build-A
printf 'Starting %s\n' "$phase"
source /tmp/baylee-b8-p3-build-A.sh
python3 - <<'PY'
from pathlib import Path
s=Path('/tmp/baylee-sba-benchmark/sba-tests-A.log').read_text()
assert '1 passed; 1 failed;' in s, s[-4000:]
assert 'marked_survivors_reset_without_changing_other_state_or_the_checkpoint ... ok' in s
PY
phase=apply-guard
printf 'Starting %s\n' "$phase"
python3 - <<'PY'
from pathlib import Path
p=Path('crates/baylee-engine/src/sba.rs')
a=Path('/tmp/baylee-sba-benchmark/A/sba.rs').read_text()
assert p.read_text()==a
old='''    for id in state.zones.list(ZoneLocation::Battlefield).clone() {
        if let Some(obj) = state.object_mut(id) {
            obj.deathtouched = false;
        }
    }'''
new='''    for id in state.zones.list(ZoneLocation::Battlefield).clone() {
        if state.object(id).is_some_and(|obj| obj.deathtouched)
            && let Some(obj) = state.object_mut(id)
        {
            obj.deathtouched = false;
        }
    }'''
assert a.count(old)==1
p.write_text(a.replace(old,new))
PY
phase=build-B-and-green
printf 'Starting %s\n' "$phase"
source /tmp/baylee-b8-p3-build-B.sh
python3 - <<'PY'
from pathlib import Path
import hashlib,json
out=Path('/tmp/baylee-sba-benchmark')
a=json.loads((out/'engine-sources-A.json').read_text())
b={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(Path('crates/baylee-engine').rglob('*')) if p.is_file()}
(out/'engine-sources-B.json').write_text(json.dumps(b,indent=2)+'\n')
assert a.keys()==b.keys()
assert [p for p in a if a[p]!=b[p]]==['crates/baylee-engine/src/sba.rs']
for variant in ('A','B'):
    for name in ('basics','xtask'):
        libraries=(out/variant/(name+'.libs.txt')).read_text().splitlines()[1:]
        assert libraries
        assert all(line.strip().startswith(('/usr/lib/','/System/Library/')) for line in libraries), libraries
(out/'identity-checks.txt').write_text('Only SBA differs in engine sources; compiler/profile/bench inputs match; no mutable Baylee dylibs; A/B/original pool byte-identical.\n')
PY
phase=targeted-tests-and-lint
printf 'Starting %s\n' "$phase"
for filter in sba::tests deathtouch phas state::tests arena::tests; do
    clean "$TOOL_BIN/cargo" test --locked --offline -p baylee-engine --lib "$filter" -- --nocapture > "$OUT/test-${filter//::/-}.log" 2>&1
done
clean "$TOOL_BIN/rustfmt" --edition 2024 --check --config skip_children=true crates/baylee-engine/src/sba.rs crates/baylee-engine/src/sba/tests/cow_reset.rs crates/baylee-engine/benches/basics.rs > "$OUT/fmt.log" 2>&1
clean "$TOOL_BIN/cargo" clippy --locked --offline -p baylee-engine --lib --tests --bench basics -- -D warnings > "$OUT/clippy.log" 2>&1
phase=timings
printf 'Starting %s\n' "$phase"
source /tmp/baylee-b8-p3-measure.sh
phase=timings-ready-for-review
printf 'Starting %s\n' "$phase"
touch "$OUT/timings-ready"
# Keep the same outer lock while the controller inspects completed timings.
# The controller supplies only bounded repeats/profiling, never another lock.
for ((attempt=0;attempt<120;attempt++)); do
    if test -f "$OUT/continue-after-review.sh"; then break; fi
    sleep 5
done
test -f "$OUT/continue-after-review.sh"
phase=reviewed-repeats-and-profile
# shellcheck source=/dev/null
source "$OUT/continue-after-review.sh"
phase=complete
printf 'Starting %s\n' "$phase"
