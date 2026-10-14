set -euo pipefail
source /tmp/baylee-b8-p3-common.sh
set +e
clean "$TOOL_BIN/cargo" test --locked --offline -p baylee-engine --lib sba::tests::cow_reset -- --nocapture > "$OUT/sba-tests-A.log" 2>&1
red_exit=$?
set -e
test "$red_exit" -ne 0
python3 - <<'CHECK'
import re
from pathlib import Path
s=Path('/tmp/baylee-sba-benchmark/sba-tests-A.log').read_text()
assert 'test result: FAILED. 1 passed; 1 failed;' in s and re.search(r'^untouched arena chunk [0-9]+ was copied$', s, re.MULTILINE), s[-4000:]
CHECK
python3 - <<'PY'
from pathlib import Path
import subprocess
path = 'crates/baylee-engine/src/sba.rs'
head = subprocess.check_output(['git', 'show', 'HEAD:' + path], text=True)
current = Path(path).read_text()
assert current.split('#[cfg(test)]')[0] == head.split('#[cfg(test)]')[0], 'A is already patched'
PY
build_variant A
cmp /tmp/baylee-b8-pool-before.json "$OUT/A/pool.txt"
cat "$OUT/A/pool.log"
