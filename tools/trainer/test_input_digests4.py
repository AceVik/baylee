"""Stat receipts never substitute a stale sealed digest for changed inputs."""
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from input_digests4 import InputChangedError, InputDigestCache, STRATEGY
from run_guard4 import run_job


class InputDigestTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.path = self.root / "input.bin"
        self.path.write_bytes(b"original" * 100)
        self.cache = InputDigestCache()

    def test_unchanged_second_lookup_reads_zero_bytes_and_receipt_is_immutable(self):
        first = self.cache.receipt(self.path)
        before = self.cache.statistics()
        with patch("input_digests4.os.open", side_effect=AssertionError("cache hit opened bytes")):
            self.assertEqual(self.cache.sha256(self.path), hashlib.sha256(self.path.read_bytes()).hexdigest())
        self.assertEqual(self.cache.statistics()["bytes_read"], before["bytes_read"])
        self.assertEqual(self.cache.statistics()["hits"], 1)
        receipt_json = first.as_dict()
        receipt_json["identity"]["size"] = -1
        self.assertEqual(self.cache.receipt(self.path), first)

    def test_same_length_edit_and_reset_mtime_still_rehashes_ctime(self):
        first = self.cache.sha256(self.path)
        identity = self.path.stat()
        self.path.write_bytes(b"modified" * 100)
        os.utime(self.path, ns=(identity.st_atime_ns, identity.st_mtime_ns))
        self.assertEqual(self.path.stat().st_size, identity.st_size)
        self.assertEqual(self.path.stat().st_mtime_ns, identity.st_mtime_ns)
        self.assertNotEqual(self.path.stat().st_ctime_ns, identity.st_ctime_ns)
        self.assertNotEqual(self.cache.sha256(self.path), first)
        self.assertEqual(self.cache.statistics()["hashes"], 2)

    def test_atomic_replacement_with_preserved_size_and_mtime(self):
        first = self.cache.sha256(self.path)
        identity = self.path.stat()
        replacement = self.root / "new.bin"
        replacement.write_bytes(b"replaced" * 100)
        os.utime(replacement, ns=(identity.st_atime_ns, identity.st_mtime_ns))
        os.replace(replacement, self.path)
        self.assertNotEqual(self.path.stat().st_ino, identity.st_ino)
        self.assertNotEqual(self.cache.sha256(self.path), first)
        self.assertEqual(self.cache.statistics()["hashes"], 2)

    def test_symlink_retarget_and_same_content_different_target_are_not_hits(self):
        link = self.root / "alias"
        other = self.root / "other.bin"
        other.write_bytes(b"new file")
        link.symlink_to(self.path)
        first = self.cache.sha256(link)
        link.unlink()
        link.symlink_to(other)
        self.assertNotEqual(self.cache.sha256(link), first)
        copy = self.root / "copy.bin"
        copy.write_bytes(other.read_bytes())
        link.unlink()
        link.symlink_to(copy)
        self.assertEqual(self.cache.sha256(link), hashlib.sha256(copy.read_bytes()).hexdigest())
        self.assertEqual(self.cache.statistics()["hashes"], 3)
        self.assertEqual(self.cache.statistics()["hits"], 0)

    def mutate_during_hash(self, mutate):
        factory = hashlib.sha256
        fired = False

        class Hash:
            def __init__(self):
                self.inner = factory()

            def update(self, chunk):
                nonlocal fired
                self.inner.update(chunk)
                if not fired:
                    fired = True
                    mutate()

            def hexdigest(self):
                return self.inner.hexdigest()

        return patch("input_digests4.hashlib.sha256", Hash)

    def test_in_place_change_during_hash_rejects_and_does_not_cache(self):
        original = self.path.stat()
        def mutate():
            self.path.write_bytes(b"modified" * 100)
            os.utime(self.path, ns=(original.st_atime_ns, original.st_mtime_ns))
        with self.mutate_during_hash(mutate), self.assertRaises(InputChangedError):
            self.cache.sha256(self.path)
        self.assertEqual(self.cache.sha256(self.path), hashlib.sha256(self.path.read_bytes()).hexdigest())
        self.assertEqual(self.cache.statistics()["hashes"], 2)
        self.assertEqual(self.cache.statistics()["hits"], 0)

    def test_replacement_and_symlink_retarget_during_read_reject(self):
        other = self.root / "other"
        other.write_bytes(b"replaced")
        def replace():
            os.replace(other, self.path)
        with self.mutate_during_hash(replace), self.assertRaises(InputChangedError):
            self.cache.sha256(self.path)
        link = self.root / "alias"
        link.symlink_to(self.path)
        other.write_bytes(b"new target")
        def retarget():
            link.unlink()
            link.symlink_to(other)
        with self.mutate_during_hash(retarget), self.assertRaises(InputChangedError):
            self.cache.sha256(link)

    def test_cache_hit_path_replacement_during_recheck_is_rejected(self):
        self.cache.sha256(self.path)
        from input_digests4 import _snapshot
        calls = 0
        def snapshot(path):
            nonlocal calls
            calls += 1
            if calls == 2:
                self.path.write_bytes(b"modified" * 100)
            return _snapshot(path)
        with patch("input_digests4._snapshot", side_effect=snapshot), self.assertRaises(InputChangedError):
            self.cache.sha256(self.path)

    def test_verify_preserves_expected_seal_and_rejects_changed_input(self):
        expected = {self.path: self.cache.sha256(self.path)}
        original = dict(expected)
        self.cache.verify(expected)
        self.path.write_bytes(b"different")
        with self.assertRaisesRegex(ValueError, "Sealed input digest mismatch"):
            self.cache.verify(expected)
        self.assertEqual(expected, original)
        with self.assertRaises(ValueError):
            self.cache.sha256(self.root)

    def test_run_job_cache_is_explicit_and_reused_only_in_process(self):
        command = [sys.executable, "-c", "pass"]
        self.assertEqual(run_job(self.root / "first", command, 1, inputs=[self.path], digest_cache=self.cache), 0)
        before = self.cache.statistics()
        self.assertEqual(run_job(self.root / "second", command, 1, inputs=[self.path], digest_cache=self.cache), 0)
        value = json.loads((self.root / "second/job.json").read_text())
        self.assertEqual(value["input_digest_strategy"], STRATEGY)
        self.assertEqual(value["input_digest_cache_activity"], dict(hits=2, hashes=0, bytes_read=0))
        self.assertEqual(value["inputs"][str(self.path)], hashlib.sha256(self.path.read_bytes()).hexdigest())
        self.assertEqual(self.cache.statistics()["bytes_read"], before["bytes_read"])
        self.assertEqual(run_job(self.root / "default", command, 1, inputs=[self.path]), 0)
        normal = json.loads((self.root / "default/job.json").read_text())
        self.assertNotIn("input_digest_strategy", normal)
        self.assertNotIn("input_digest_receipts", normal)


if __name__ == "__main__":
    unittest.main()
