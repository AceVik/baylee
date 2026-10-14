set -euo pipefail
source /tmp/baylee-b8-p3-common.sh
python3 - <<'PY'
from pathlib import Path
a = Path('/tmp/baylee-sba-benchmark/A/sba.rs').read_text()
b = Path('crates/baylee-engine/src/sba.rs').read_text()
old = '''    for id in state.zones.list(ZoneLocation::Battlefield).clone() {
        if let Some(obj) = state.object_mut(id) {
            obj.deathtouched = false;
        }
    }'''
new = '''    for id in state.zones.list(ZoneLocation::Battlefield).clone() {
        if state.object(id).is_some_and(|obj| obj.deathtouched)
            && let Some(obj) = state.object_mut(id)
        {
            obj.deathtouched = false;
        }
    }'''
assert a.count(old) == 1
assert b == a.replace(old, new), 'B must differ only by the agreed SBA guard'
PY
build_variant B
cmp "$OUT/A/rustc.txt" "$OUT/B/rustc.txt"
cmp "$OUT/A/cargo.txt" "$OUT/B/cargo.txt"
cmp "$OUT/A/inputs.sha256" "$OUT/B/inputs.sha256"
cmp "$OUT/A/pool.txt" "$OUT/B/pool.txt"
cmp /tmp/baylee-b8-pool-before.json "$OUT/B/pool.txt"
cat "$OUT/B/pool.log"
clean "$TOOL_BIN/cargo" test --locked --offline -p baylee-engine --lib \
    sba::tests::cow_reset -- --nocapture > "$OUT/sba-tests-B.log" 2>&1
