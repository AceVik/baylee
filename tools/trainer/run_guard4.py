"""Run one reproducible training/collection job in an isolated process group.

The durable manifest exists before the child starts. A crash, deadline or
memory limit preserves stdout, stderr, arguments and source identity. Re-running
a successful job is a no-op; a failed job needs a new job directory. No engine
repair or retry changes the rules/version of a running dataset.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import time


def atomic_json(path, value):
    tmp = path.with_suffix(".partial")
    with tmp.open("w") as f:
        json.dump(value, f, indent=2)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, path)


def group_pss_bytes(group):
    total = 0
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            # comm can contain spaces/parentheses; fields follow its last ')'.
            stat = (entry / "stat").read_text().rsplit(")", 1)[1].split()
            if int(stat[2]) != group:
                continue
            for line in (entry / "smaps_rollup").read_text().splitlines():
                if line.startswith("Pss:"):
                    total += int(line.split()[1]) * 1024
                    break
        except (OSError, ValueError, IndexError):
            pass  # A process can exit while its /proc files are being read.
    return total


def run_job(directory, command, timeout, memory_gib=48, inputs=()):
    directory = Path(directory).resolve()
    directory.mkdir(parents=True, exist_ok=True)
    manifest = directory / "job.json"
    if manifest.exists():
        previous = json.loads(manifest.read_text())
        if previous["command"] == command and previous["status"] == "complete":
            return 0
        raise FileExistsError(f"Preserving existing job: {manifest}; use a new attempt directory")
    def git(*args):
        return subprocess.check_output(["git", *args], stderr=subprocess.DEVNULL)
    revision = git("rev-parse", "HEAD").decode().strip()
    diff = git("diff", "--binary", "HEAD")
    (directory / "source.patch").write_bytes(diff)
    executable = Path(shutil.which(command[0]) or command[0]).resolve()
    with executable.open("rb") as executable_file:
        executable_sha256 = hashlib.file_digest(executable_file, "sha256").hexdigest()
    state = {"command": command, "cwd": os.getcwd(), "revision": revision,
             "executable": str(executable), "executable_sha256": executable_sha256,
             "tracked_diff_sha256": hashlib.sha256(diff).hexdigest(),
             "inputs": {str(p): hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in inputs},
             "status": "starting", "started_at_unix": time.time(),
             "timeout_seconds": timeout, "memory_limit_gib": memory_gib}
    atomic_json(manifest, state)
    with (directory / "stdout.log").open("wb") as stdout, (directory / "stderr.log").open("wb") as stderr:
        child = subprocess.Popen(command, stdout=stdout, stderr=stderr, start_new_session=True)
        state.update(pid=child.pid, status="running")
        atomic_json(manifest, state)
        start = time.monotonic()
        reason = None
        peak = 0
        try:
            while child.poll() is None:
                pss = group_pss_bytes(child.pid)
                peak = max(peak, pss)
                if timeout > 0 and time.monotonic() - start > timeout:
                    reason = "deadline"
                if memory_gib > 0 and pss > memory_gib * 1024**3:
                    reason = "memory_limit"
                if reason:
                    os.killpg(child.pid, signal.SIGTERM)
                    try:
                        child.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait()
                    break
                time.sleep(.25 if timeout < 10 else 2)
        finally:
            # A failed parent can leave workers alive. They belong to this
            # dedicated group even after the group leader has exited.
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            child.wait()
        state.update(status="complete" if child.returncode == 0 and reason is None else "quarantined",
                     returncode=child.returncode, reason=reason,
                     elapsed_seconds=time.monotonic() - start, peak_pss_bytes=peak,
                     finished_at_unix=time.time())
        atomic_json(manifest, state)
    return 0 if state["status"] == "complete" else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--job", required=True)
    parser.add_argument("--timeout", type=float, default=3600)
    parser.add_argument("--memory-gib", type=float, default=48)
    parser.add_argument("--input", action="append", default=[])
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("a command after -- is required")
    raise SystemExit(run_job(args.job, command, args.timeout, args.memory_gib, args.input))


if __name__ == "__main__":
    main()
