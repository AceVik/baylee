"""The watchdog must preserve failures and stop orphaned job workers."""
import json
import os
import signal
import subprocess
import hashlib
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

import run_guard4 as guard
from run_guard4 import run_job


class GuardContracts(unittest.TestCase):
    def test_deadline_is_quarantined_and_success_is_reusable(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            command = [sys.executable, "-c", "print('done')"]
            self.assertEqual(run_job(root / "ok", command, 10), 0)
            self.assertEqual(run_job(root / "ok", command, 10), 0)
            self.assertEqual(run_job(root / "late", [sys.executable, "-c", "import time; time.sleep(10)"], .1), 1)
            status = json.loads((root / "late" / "job.json").read_text())
            self.assertEqual(status["reason"], "deadline")
            self.assertEqual(status["status"], "quarantined")

    def test_input_hashes_stream_without_read_bytes_and_keep_manifest_contract(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            small, large = root / "abc.bin", root / "multi-buffer.bin"
            small.write_bytes(b"abc")
            block = bytes(range(256)) * 4096  # Exceeds file_digest's read buffer.
            large.write_bytes(block)
            expected = {str(small): "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
                        str(large): hashlib.sha256(block).hexdigest()}
            with patch.object(Path, "read_bytes", side_effect=AssertionError("Whole-file allocation forbidden")):
                result = run_job(root / "streamed", [sys.executable, "-c", "pass"], 1,
                                 inputs=[small, str(large)])
            self.assertEqual(result, 0)
            manifest = json.loads((root / "streamed/job.json").read_text())
            self.assertEqual(manifest["inputs"], expected)
            self.assertEqual(manifest["status"], "complete")

    def test_reaped_leader_and_lost_wait_ownership_never_signal_a_group(self):
        child = subprocess.Popen([sys.executable, "-c", "pass"], start_new_session=True)
        child.wait(timeout=5)
        with patch.object(guard.os, "killpg") as kill:
            with self.assertRaisesRegex(RuntimeError, "already reaped"):
                guard._reap_group(child)
            # Simulate a different reaper consuming waitpid without Popen knowing.
            child.returncode = None
            try:
                with self.assertRaisesRegex(RuntimeError, "ownership lost"):
                    guard._signal_owned(child, signal.SIGKILL)
            finally:
                child.returncode = 0
            kill.assert_not_called()

    def test_monitor_exception_reaps_child_and_preserves_quarantine(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with patch.object(guard, "group_pss_bytes", side_effect=RuntimeError("monitor failed")):
                with self.assertRaisesRegex(RuntimeError, "monitor failed"):
                    run_job(root / "job", [sys.executable, "-c", "import time; time.sleep(60)"], 1)
            state = json.loads((root / "job/job.json").read_text())
            self.assertEqual(state["status"], "quarantined")
            self.assertEqual(state["reason"], "exception")
            with self.assertRaises(ChildProcessError):
                os.waitid(os.P_PID, state["pid"], os.WEXITED | os.WNOHANG)

    def test_success_without_descendants_never_signals_a_group(self):
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(guard, "_signal_owned", side_effect=AssertionError("No live group to signal")):
                self.assertEqual(run_job(Path(tmp) / "job", [sys.executable, "-c", "pass"], 1), 0)

    def test_memory_limit_reaps_before_return(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with patch.object(guard, "group_pss_bytes", return_value=2 * 1024**3):
                self.assertEqual(run_job(root / "job", [sys.executable, "-c", "import time; time.sleep(60)"], 1, memory_gib=1), 1)
            state = json.loads((root / "job/job.json").read_text())
            self.assertEqual(state["reason"], "memory_limit")
            self.assertIsNotNone(state["returncode"])

    def test_ignoring_grandchild_is_killed_before_leader_reap(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            pidfile = root / "worker.pid"
            worker = "import signal,os,pathlib,time,sys; signal.signal(signal.SIGTERM,signal.SIG_IGN); pathlib.Path(sys.argv[1]).write_text(str(os.getpid())); time.sleep(60)"
            leader = "import subprocess,sys,pathlib,time; subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[2]]); p=pathlib.Path(sys.argv[2]); exec('while not p.exists(): time.sleep(.01)')"
            signals = []
            original_signal, original_reap = guard._signal_owned, guard._reap_group
            def observed(child, sig):
                self.assertIsNone(child.returncode)
                # WNOWAIT remains available even though the leader has exited.
                os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                signals.append(sig)
                original_signal(child, sig)
            with patch.object(guard, "_signal_owned", side_effect=observed), patch.object(
                    guard, "_reap_group", side_effect=lambda child: original_reap(child, grace=.2)):
                self.assertEqual(run_job(root / "job", [sys.executable, "-c", leader, worker, str(pidfile)], 2), 0)
            self.assertIn(signal.SIGKILL, signals)
            pid = int(pidfile.read_text())
            self.assertNotIn(pid, guard._active_group_members(json.loads((root / "job/job.json").read_text())["pid"]))

    def test_real_sigterm_interrupt_reaps_owned_job(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            code = "import sys; import run_guard4; run_guard4.run_job(sys.argv[1], [sys.executable,'-c','import time; time.sleep(60)'], 30)"
            controller = subprocess.Popen([sys.executable, "-c", code, str(root / "job")],
                env={**os.environ, "PYTHONPATH": str(Path(guard.__file__).parent)},
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            try:
                deadline = time.monotonic() + 5
                manifest = root / "job/job.json"
                while not manifest.exists() or json.loads(manifest.read_text())["status"] != "running":
                    if time.monotonic() >= deadline:
                        self.fail("controller did not start")
                    time.sleep(.01)
                controller.send_signal(signal.SIGTERM)
                controller.wait(timeout=7)
                state = json.loads(manifest.read_text())
                self.assertEqual(state["status"], "quarantined")
                self.assertEqual(state["reason"], "interrupted")
                self.assertEqual(guard._active_group_members(state["pid"]), [])
            finally:
                if controller.poll() is None:
                    controller.kill()
                controller.wait(timeout=5)

    def test_parent_exit_cannot_leave_a_worker_running(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            pid_file = root / "worker.pid"
            command = [sys.executable, "-c",
                       "import subprocess,sys,pathlib; "
                       "p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); "
                       "pathlib.Path(sys.argv[1]).write_text(str(p.pid))", str(pid_file)]
            self.assertEqual(run_job(root / "parent", command, 1), 0)
            pid = int(pid_file.read_text())
            for _ in range(20):
                try:
                    state = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[0]
                    if state == "Z":
                        return
                except FileNotFoundError:
                    return
                time.sleep(.05)
            self.fail("Worker still runs after its parent exited")


if __name__ == "__main__":
    unittest.main()
