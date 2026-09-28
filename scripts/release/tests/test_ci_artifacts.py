"""Exercise provenance failures before any executable can reach the signer."""

import copy
import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "ci_artifacts", Path(__file__).parents[1] / "ci_artifacts.py"
)
ci = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci)
REPO = "AceVik/baylee"
SHA = "a" * 40
RUN = {
    "id": 123,
    "head_sha": SHA,
    "head_branch": "main",
    "event": "push",
    "status": "completed",
    "conclusion": "success",
    "path": ".github/workflows/ci.yml",
    "repository": {"full_name": REPO},
    "head_repository": {"full_name": REPO},
}
ARTIFACTS = [{"name": f"client-{target}", "expired": False} for target in ci.TARGETS]


class Provenance(unittest.TestCase):
    def setUp(self):
        self.env = patch.dict(
            os.environ, GITHUB_REPOSITORY=REPO, GITHUB_SHA=SHA, GITHUB_RUN_ID="123"
        )
        self.env.start()
        self.addCleanup(self.env.stop)

    def test_successful_main_only(self):
        self.assertTrue(ci.trusted_run(RUN, REPO, SHA))
        for field, value in [
            ("event", "pull_request"),
            ("event", "workflow_dispatch"),
            ("head_branch", "feature"),
            ("head_sha", "b" * 40),
            ("status", "in_progress"),
            ("conclusion", "failure"),
            ("conclusion", "cancelled"),
            ("path", ".github/workflows/evil.yml"),
            ("repository", {"full_name": "fork/baylee"}),
            ("head_repository", {"full_name": "fork/baylee"}),
        ]:
            with self.subTest(field=field, value=value):
                self.assertFalse(ci.trusted_run(dict(RUN, **{field: value}), REPO, SHA))
        self.assertFalse(ci.trusted_run({}, REPO, SHA))

    def test_verify_run_requires_complete_live_artifact_set(self):
        with patch.object(ci, "api", side_effect=[RUN, {"artifacts": ARTIFACTS}]):
            ci.verify_run("123")
        broken = [
            ARTIFACTS[:-1],
            ARTIFACTS + [ARTIFACTS[0]],
            [dict(a, expired=True) for a in ARTIFACTS],
        ]
        for artifacts in broken:
            with (
                self.subTest(artifacts=artifacts),
                patch.object(ci, "api", side_effect=[RUN, {"artifacts": artifacts}]),
                self.assertRaises(ValueError),
            ):
                ci.verify_run("123")
        with (
            patch.object(ci, "api", return_value=dict(RUN, event="pull_request")),
            self.assertRaises(ValueError),
        ):
            ci.verify_run("123")
        with self.assertRaises(ValueError):
            ci.verify_run("../123")

    def test_resolve_rejects_pr_or_wrong_commit_even_if_api_returns_it(self):
        for run in [dict(RUN, event="pull_request"), dict(RUN, head_sha="b" * 40)]:
            with (
                patch.object(ci, "api", return_value={"workflow_runs": [run]}),
                self.assertRaises(ValueError),
            ):
                ci.resolve()
        with patch.object(
            ci,
            "api",
            side_effect=[{"workflow_runs": [RUN]}, RUN, {"artifacts": ARTIFACTS}],
        ):
            self.assertEqual(ci.resolve(), "123")

    def package(self, directory, target):
        archive, manifest = ci.names("0.1.0-beta.4", target)
        (directory / archive).write_bytes(b"packaged executable")
        ci.write(directory, "0.1.0-beta.4", target)
        return archive, manifest

    def test_all_platforms_roundtrip(self):
        for target in ci.TARGETS:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as tmp:
                directory = Path(tmp)
                self.package(directory, target)
                ci.verify(directory, "0.1.0-beta.4", target, "123")

    def test_corruption_or_wrong_identity_is_refused(self):
        target = "x86_64-pc-windows-msvc"
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            archive, manifest = self.package(directory, target)
            original = json.loads((directory / manifest).read_text())
            for field, value in [
                ("commit", "b" * 40),
                ("version", "0.1.0-beta.3"),
                ("target", "aarch64-pc-windows-msvc"),
                ("run_id", "124"),
                ("repository", "fork/baylee"),
                ("schema", 2),
                ("archive", "../payload.zip"),
                ("sha256", "0" * 64),
            ]:
                with self.subTest(field=field):
                    data = copy.deepcopy(original)
                    data[field] = value
                    (directory / manifest).write_text(json.dumps(data))
                    with self.assertRaises(ValueError):
                        ci.verify(directory, "0.1.0-beta.4", target, "123")
            (directory / manifest).write_text(json.dumps(original))
            (directory / archive).write_bytes(b"tampered executable")
            with self.assertRaises(ValueError):
                ci.verify(directory, "0.1.0-beta.4", target, "123")

    def test_missing_extra_and_rewritten_checksum_are_refused(self):
        target = "aarch64-apple-darwin"
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            archive, _ = self.package(directory, target)
            extra = directory / "unexpected.exe"
            extra.write_bytes(b"extra")
            with self.assertRaises(ValueError):
                ci.verify(directory, "0.1.0-beta.4", target, "123")
            extra.unlink()
            checksum = directory / (archive + ".sha256")
            checksum.write_text("changed")
            with self.assertRaises(ValueError):
                ci.verify(directory, "0.1.0-beta.4", target, "123")
            checksum.unlink()
            with self.assertRaises(ValueError):
                ci.verify(directory, "0.1.0-beta.4", target, "123")

    def test_paths_are_not_versions(self):
        for version in ["../evil", "1.2.3\nextra", "1.2.3/a", "$(id)"]:
            with self.assertRaises(ValueError):
                ci.names(version, "aarch64-apple-darwin")


if __name__ == "__main__":
    unittest.main()
