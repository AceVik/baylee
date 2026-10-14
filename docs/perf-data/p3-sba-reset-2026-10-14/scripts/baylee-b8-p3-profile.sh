set -euo pipefail
source /tmp/baylee-b8-p3-common.sh
quiet_machine
clean python3 - <<'PY'
import os, signal, subprocess, time
from pathlib import Path
root = Path('/tmp/baylee-sba-benchmark')
def interrupted(signum, frame):
    raise KeyboardInterrupt
signal.signal(signal.SIGTERM, interrupted)
for variant in ('A', 'B'):
    if os.getloadavg()[0] >= 10:
        raise SystemExit('refusing profile at load >= 10')
    out = root / 'profiles' / variant
    out.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, CRITERION_HOME=str(out / 'criterion'))
    logpath = out / 'benchmark.log'
    with logpath.open('w') as log:
        proc = subprocess.Popen([
            str(root / variant / 'basics'), '--bench', '--noplot',
            '--profile-time', '30', '--exact', 'combat/attack_to_blocks_900',
        ], env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            deadline = time.monotonic() + 60
            while 'Profiling for' not in logpath.read_text():
                if proc.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError('profile marker absent; inspect benchmark.log')
                time.sleep(0.1)
            subprocess.run([
                '/usr/bin/sample', str(proc.pid), '10', '1', '-mayDie',
                '-file', str(out / 'sample.txt'),
            ], check=True, timeout=30)
            if proc.wait(timeout=90) != 0:
                raise RuntimeError('profile process failed')
        finally:
            if proc.poll() is None:
                os.killpg(proc.pid, signal.SIGTERM)
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(proc.pid, signal.SIGKILL)
                    proc.wait()
PY
