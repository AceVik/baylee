"""CPU-only watcher tests: no trainer imports and no actual Codex queue."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from watch_run4 import arm, check, locked, process_identity, queue_wake


class WatchTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.controller = {'pid': 8539, 'start_ticks': 42, 'boot_id': 'boot'}
        self.live = self.controller.copy()
        self.clock = 100
        self.calls = []
        self.thread = '019a18f5-1111-7111-8111-111111111111'
        self.status('training')
        arm(self.root, self.thread, ['complete'], ['paired.json'], pid=8539,
            identity=lambda pid: self.live)

    def status(self, phase):
        (self.root / 'status.json').write_text(json.dumps({'phase': phase}))

    def queue(self, thread, message):
        self.calls.append((thread, message))
        return 'queued receipt\n'

    def tick(self, seconds=30, queue=None):
        self.clock += seconds
        return check(self.root, now=lambda: self.clock,
                     identity=lambda pid: self.live, queue=queue or self.queue)

    def test_success_artifact_receipt_and_no_repeat(self):
        self.status('complete')
        (self.root / 'paired.json').write_text('{}')
        self.assertEqual(self.tick(), 'delivered')
        self.assertEqual(self.calls[0][0], self.thread)
        self.assertIn('"phase": "complete"', self.calls[0][1])
        delivered = json.loads((self.root / 'watchdog/delivered.json').read_text())
        self.assertEqual(delivered['queue_receipt'], 'queued receipt\n')
        self.assertEqual(self.tick(), 'delivered')
        self.assertEqual(len(self.calls), 1)

    def test_running_throttles_and_stays_quiet(self):
        self.assertEqual(self.tick(), 'running')
        self.assertEqual(self.tick(1), 'waiting')
        self.assertEqual(self.tick(), 'running')
        self.assertFalse(self.calls)

    def test_death_has_grace_and_terminal_can_lag(self):
        self.live = None
        self.assertEqual(self.tick(), 'confirming-exit')
        self.status('complete')
        (self.root / 'paired.json').write_text('{}')
        self.assertEqual(self.tick(), 'delivered')
        self.assertIn('experiment completed', self.calls[0][1])

    def test_pid_reuse_and_reboot_are_exits(self):
        for field, value in [('start_ticks', 43), ('boot_id', 'new-boot')]:
            with self.subTest(field=field):
                self.live = {**self.controller, field: value}
                self.assertEqual(self.tick(), 'confirming-exit')
                self.live = self.controller.copy()
                self.assertEqual(self.tick(), 'running')
        self.live = {**self.controller, 'start_ticks': 43}
        self.tick()
        self.assertEqual(self.tick(), 'delivered')
        self.assertIn('controller exited', self.calls[0][1])

    def test_missing_and_corrupt_status_require_persistence(self):
        (self.root / 'status.json').unlink()
        self.assertEqual(self.tick(), 'running')
        (self.root / 'status.json').write_text('{')
        self.assertEqual(self.tick(), 'running')
        self.assertEqual(self.tick(), 'delivered')
        self.assertIn('persistent unreadable status', self.calls[0][1])

    def test_status_recovery_resets_errors(self):
        (self.root / 'status.json').write_text('[]')
        self.tick()
        self.status('training')
        self.tick()
        (self.root / 'status.json').write_text('{')
        self.assertEqual(self.tick(), 'running')
        self.assertEqual(self.tick(), 'running')
        self.assertFalse(self.calls)

    def test_invalid_artifact_waits_for_exit_then_reports_incomplete(self):
        self.status('complete')
        (self.root / 'paired.json').write_text('')
        self.assertEqual(self.tick(), 'running')
        self.live = None
        self.assertEqual(self.tick(), 'confirming-exit')
        self.assertEqual(self.tick(), 'delivered')
        self.assertIn('incomplete terminal result', self.calls[0][1])

    def test_failure_retries_queue_with_stable_event_and_backoff(self):
        self.status('failed')
        def fail(thread, message):
            self.calls.append((thread, message))
            raise subprocess.TimeoutExpired('fake', 30)
        self.assertEqual(self.tick(queue=fail), 'queue-retry')
        self.assertEqual(self.tick(), 'backoff')
        self.assertEqual(self.tick(), 'delivered')
        self.assertEqual(self.calls[0], self.calls[1])

    def test_queue_uses_exact_argv_timeout_and_captures_receipt(self):
        with patch('watch_run4.subprocess.run') as run:
            run.return_value.stdout = 'receipt'
            self.assertEqual(queue_wake(self.thread, 'message'), 'receipt')
            run.assert_called_once_with(
                ['/usr/local/bin/codex', 'queue', '--thread', self.thread,
                 '--message', 'message'], check=True, timeout=30,
                stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)

    def test_nonblocking_lock(self):
        with locked(self.root / 'watchdog') as acquired:
            self.assertTrue(acquired)
            self.assertEqual(self.tick(), 'busy')
        self.assertFalse(self.calls)

    def test_arm_is_idempotent_and_preserves_delivery(self):
        self.status('failed')
        self.tick()
        arm(self.root, self.thread, ['complete'], ['paired.json'], pid=8539,
            identity=lambda pid: self.live)
        self.assertEqual(self.tick(), 'delivered')
        self.assertEqual(len(self.calls), 1)

    def test_proc_parser_spaces_zombie_and_absent(self):
        proc = self.root / 'proc'
        (proc / '12').mkdir(parents=True)
        (proc / 'sys/kernel/random').mkdir(parents=True)
        (proc / 'sys/kernel/random/boot_id').write_text('boot\n')
        fields = ['S'] + ['0'] * 18 + ['1234']
        stat = proc / '12/stat'
        stat.write_text('12 (name with ) spaces) ' + ' '.join(fields))
        self.assertEqual(process_identity(12, proc),
                         {'pid': 12, 'boot_id': 'boot', 'start_ticks': 1234})
        fields[0] = 'Z'
        stat.write_text('12 (zombie) ' + ' '.join(fields))
        self.assertIsNone(process_identity(12, proc))
        self.assertIsNone(process_identity(13, proc))


if __name__ == '__main__':
    unittest.main()
