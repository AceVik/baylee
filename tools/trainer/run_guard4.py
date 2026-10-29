"""Run one reproducible training/collection job in an isolated process group.

The durable manifest exists before the child starts. A crash, deadline or
memory limit preserves stdout, stderr, arguments and source identity. Re-running
a successful job is a no-op; a failed job needs a new job directory. No engine
repair or retry changes the rules/version of a running dataset.
"""
from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import time
import threading


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


def input_sha256(path):
    """Hash immutable inputs without allocating their potentially GiB-sized bytes."""
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()



def _leader_status(child):
    """Observe, never reap: the owned zombie pins its PID and process group."""
    if child.returncode is not None:
        raise RuntimeError("Job leader already reaped; process-group ownership lost")
    try:
        return os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
    except ChildProcessError as error:
        raise RuntimeError("Job leader ownership lost; refusing group signal") from error


def _active_group_members(group):
    """Exclude zombies, which cannot keep job resources or spawn children."""
    result = []
    for path in Path("/proc").glob("[0-9]*/stat"):
        try:
            stat = path.read_text().rsplit(")", 1)[1].split()
            if int(stat[2]) == group and stat[0] not in ("Z", "X"):
                result.append(int(path.parent.name))
        except (OSError, ValueError, IndexError):
            pass
    return result


def _signal_owned(child, sig):
    """Never signal a PID/PGID after losing direct-child ownership."""
    _leader_status(child)
    try:
        os.killpg(child.pid, sig)
    except ProcessLookupError:
        pass


@contextlib.contextmanager
def _signals(interrupt=False):
    """Cleanup survives a second interruption; restore the caller's handlers."""
    if threading.current_thread() is not threading.main_thread():
        yield []
        return
    previous = {sig: signal.getsignal(sig) for sig in (signal.SIGTERM, signal.SIGINT)}
    pending = []
    def interrupted(signum, frame):
        # Defer the exception until Popen has returned and the child is owned.
        pending.append(signum)
    try:
        for sig in previous:
            signal.signal(sig, interrupted if interrupt else signal.SIG_IGN)
        yield pending
    finally:
        for sig, handler in previous.items():
            signal.signal(sig, handler)


def _reap_group(child, grace=5):
    """Keep the leader unreaped until every owned group member has stopped."""
    with _signals():
        _leader_status(child)
        if _active_group_members(child.pid):
            _signal_owned(child, signal.SIGTERM)
            deadline = time.monotonic() + grace
            while _active_group_members(child.pid) and time.monotonic() < deadline:
                time.sleep(.01)
            if _active_group_members(child.pid):
                _signal_owned(child, signal.SIGKILL)
        deadline = time.monotonic() + grace
        while _leader_status(child) is None or _active_group_members(child.pid):
            if time.monotonic() >= deadline:
                raise RuntimeError("Owned job group remains after cleanup; leader not reaped")
            time.sleep(.01)
        # The first and only reaping operation, after group cleanup is proven.
        child.wait(timeout=grace)

def run_job(directory, command, timeout, memory_gib=48, inputs=(), *, digest_cache=None):
    """Run with optional process-memory input receipts; None preserves full reads.

    Cache receipts are not expected seals. Controllers retaining sealed digests
    must still call digest_cache.verify(expected) before/after consumers.
    """
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
    cache_before = None if digest_cache is None else digest_cache.statistics()
    receipts = {}
    if digest_cache is None:
        with executable.open("rb") as executable_file:
            executable_sha256 = hashlib.file_digest(executable_file, "sha256").hexdigest()
        input_hashes = {str(p): input_sha256(p) for p in inputs}
    else:
        executable_receipt = digest_cache.receipt(executable)
        executable_sha256 = executable_receipt.sha256
        for path in inputs:
            receipts[str(path)] = digest_cache.receipt(path).as_dict()
        input_hashes = {path: receipt["sha256"] for path, receipt in receipts.items()}
    state = {"command": command, "cwd": os.getcwd(), "revision": revision,
             # Only known non-secret execution settings; never dump the environment.
             "runtime_environment": {key: os.environ[key] for key in
                 ("BAYLEE_EP", "CUDA_VISIBLE_DEVICES", "OMP_NUM_THREADS", "MKL_NUM_THREADS", "LD_LIBRARY_PATH")
                 if key in os.environ},
             "executable": str(executable), "executable_sha256": executable_sha256,
             "tracked_diff_sha256": hashlib.sha256(diff).hexdigest(),
             "inputs": input_hashes,
             "status": "starting", "started_at_unix": time.time(),
             "timeout_seconds": timeout, "memory_limit_gib": memory_gib}
    if digest_cache is not None:
        from input_digests4 import STRATEGY
        after = digest_cache.statistics()
        state["input_digest_strategy"] = STRATEGY
        state["input_digest_receipts"] = receipts
        state["executable_digest_receipt"] = executable_receipt.as_dict()
        state["input_digest_cache_activity"] = {key: after[key] - cache_before[key] for key in after}
    atomic_json(manifest, state)
    with _signals(interrupt=True) as interruptions, (directory / "stdout.log").open("wb") as stdout, (directory / "stderr.log").open("wb") as stderr:
        child = None
        start = time.monotonic()
        reason = None
        peak = 0
        try:
            child = subprocess.Popen(command, stdout=stdout, stderr=stderr, start_new_session=True)
            state.update(pid=child.pid, status="running")
            atomic_json(manifest, state)
            while _leader_status(child) is None:
                if interruptions:
                    raise KeyboardInterrupt("Job guard interrupted")
                pss = group_pss_bytes(child.pid)
                peak = max(peak, pss)
                if timeout > 0 and time.monotonic() - start > timeout:
                    reason = "deadline"
                if memory_gib > 0 and pss > memory_gib * 1024**3:
                    reason = "memory_limit"
                if reason:
                    break
                time.sleep(.25 if timeout < 10 else 2)
            if interruptions:
                raise KeyboardInterrupt("Job guard interrupted")
        except BaseException as error:
            reason = "interrupted" if isinstance(error, (KeyboardInterrupt, InterruptedError)) else "exception"
            raise
        finally:
            try:
                if child is not None:
                    _reap_group(child)
            except BaseException:
                reason = "cleanup_failed"
                raise
            finally:
                state.update(status="complete" if child is not None and child.returncode == 0 and reason is None else "quarantined",
                             returncode=child.returncode if child is not None else None, reason=reason,
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
