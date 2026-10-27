"""The watchdog must preserve failures and stop orphaned job workers."""
import json
import hashlib
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

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
