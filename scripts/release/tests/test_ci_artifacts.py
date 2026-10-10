"""Exercise provenance failures before any executable can reach the signer."""

import copy
import importlib.util
import json
import os
import re
import subprocess
import tarfile
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
JOBS = [
    {"name": name, "conclusion": "success"}
    for name in ci.REQUIRED_JOBS
    if name != "test"
] + [
    {"name": "test (1/2)", "conclusion": "success"},
    {"name": "test (2/2)", "conclusion": "success"},
    {"name": "build (macos-latest)", "conclusion": "skipped"},
    {"name": "packages / package (aarch64-apple-darwin)", "conclusion": "success"},
]


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
        with patch.object(
            ci, "api", side_effect=[RUN, {"jobs": JOBS}, {"artifacts": ARTIFACTS}]
        ):
            ci.verify_run("123")
        broken = [
            ARTIFACTS[:-1],
            ARTIFACTS + [ARTIFACTS[0]],
            [dict(a, expired=True) for a in ARTIFACTS],
        ]
        for artifacts in broken:
            with (
                self.subTest(artifacts=artifacts),
                patch.object(
                    ci,
                    "api",
                    side_effect=[RUN, {"jobs": JOBS}, {"artifacts": artifacts}],
                ),
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

    def test_verify_run_requires_every_release_job_to_have_run_and_passed(self):
        # A job skipped by its `if:` leaves the run green; that is the hole.
        docs_only = [
            j for j in JOBS if j["name"] in ("fmt", "test (1/2)", "test (2/2)")
        ]
        one_partition_red = [
            dict(j, conclusion="failure") if j["name"] == "test (2/2)" else j
            for j in JOBS
        ]
        skipped = [
            dict(j, conclusion="skipped") if j["name"] == "test-release" else j
            for j in JOBS
        ]
        no_test_rows = [j for j in JOBS if not j["name"].startswith("test (")]
        for jobs, named in [
            (docs_only, "test-release"),
            (one_partition_red, "test"),
            (skipped, "test-release"),
            (no_test_rows, "test"),
        ]:
            with self.subTest(named=named):
                self.assertIn(named, ci.unproven_jobs(jobs))
                with (
                    patch.object(
                        ci,
                        "api",
                        side_effect=[RUN, {"jobs": jobs}, {"artifacts": ARTIFACTS}],
                    ),
                    self.assertRaises(ValueError),
                ):
                    ci.verify_run("123")
        # `test-release` is not a row of `test`, nor the other way round.
        self.assertIn(
            "test", ci.unproven_jobs([j for j in JOBS if j["name"] == "test-release"])
        )
        self.assertEqual(ci.unproven_jobs(JOBS), [])

    def test_every_required_job_is_a_job_of_the_ci_workflow(self):
        # A renamed job would otherwise make every release fail, or a list
        # entry nobody runs would be required forever.
        workflow = (Path(__file__).parents[3] / ".github/workflows/ci.yml").read_text()
        jobs = workflow[workflow.index("\njobs:\n") :]
        ids = set(re.findall(r"^  ([A-Za-z0-9_-]+):$", jobs, re.MULTILINE))
        self.assertGreater(len(ids), len(ci.REQUIRED_JOBS))
        self.assertLessEqual(set(ci.REQUIRED_JOBS), ids)

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
            side_effect=[
                {"workflow_runs": [RUN]},
                RUN,
                {"jobs": JOBS},
                {"artifacts": ARTIFACTS},
            ],
        ):
            self.assertEqual(ci.resolve(), "123")

    def package(self, directory, target):
        archive, installers, manifest = ci.names("0.1.0-beta.4", target)
        (directory / archive).write_bytes(b"packaged executable")
        for name in installers:
            (directory / name).write_bytes(b"installer of " + name.encode())
        ci.write(directory, "0.1.0-beta.4", target)
        return archive, installers, manifest

    def test_all_platforms_roundtrip(self):
        for target in ci.TARGETS:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as tmp:
                directory = Path(tmp)
                self.package(directory, target)
                ci.verify(directory, "0.1.0-beta.4", target, "123")

    def test_every_target_has_installers_named_as_the_packaging_script_writes(self):
        self.assertEqual(set(ci.INSTALLERS), set(ci.TARGETS))
        self.assertEqual(
            {name for t in ci.TARGETS for name in ci.names("0.1.0-beta.5", t)[1]},
            {
                "Baylee-0.1.0-beta.5-aarch64.dmg",
                "Baylee-Setup-0.1.0-beta.5-x64.exe",
                "Baylee-Setup-0.1.0-beta.5-arm64.exe",
                "Baylee-0.1.0-beta.5-x86_64.AppImage",
                "Baylee-0.1.0-beta.5-aarch64.AppImage",
                "baylee_0.1.0-beta.5_amd64.deb",
                "baylee_0.1.0-beta.5_arm64.deb",
            },
        )
        # The signer and the updater match archives by these suffixes; an
        # installer must never look like one.
        for t in ci.TARGETS:
            for name in ci.names("0.1.0-beta.5", t)[1]:
                self.assertFalse(name.endswith((".zip", ".tar.gz")), name)

    def test_corruption_or_wrong_identity_is_refused(self):
        target = "x86_64-pc-windows-msvc"
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            archive, installers, manifest = self.package(directory, target)
            original = json.loads((directory / manifest).read_text())
            for field, value in [
                ("commit", "b" * 40),
                ("version", "0.1.0-beta.3"),
                ("target", "aarch64-pc-windows-msvc"),
                ("run_id", "124"),
                ("repository", "fork/baylee"),
                ("schema", 1),
                ("archive", "../payload.zip"),
                ("sha256", "0" * 64),
                ("installers", {}),
                ("installers", {installers[0]: "0" * 64}),
            ]:
                with self.subTest(field=field):
                    data = copy.deepcopy(original)
                    data[field] = value
                    (directory / manifest).write_text(json.dumps(data))
                    with self.assertRaises(ValueError):
                        ci.verify(directory, "0.1.0-beta.4", target, "123")
            (directory / manifest).write_text(json.dumps(original))
            ci.verify(directory, "0.1.0-beta.4", target, "123")
            for tampered in (archive, installers[0]):
                with self.subTest(tampered=tampered):
                    saved = (directory / tampered).read_bytes()
                    (directory / tampered).write_bytes(b"tampered executable")
                    with self.assertRaises(ValueError):
                        ci.verify(directory, "0.1.0-beta.4", target, "123")
                    (directory / tampered).write_bytes(saved)

    def test_missing_extra_and_rewritten_checksum_are_refused(self):
        target = "x86_64-unknown-linux-gnu"
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            archive, installers, _ = self.package(directory, target)
            extra = directory / "unexpected.exe"
            extra.write_bytes(b"extra")
            with self.assertRaises(ValueError):
                ci.verify(directory, "0.1.0-beta.4", target, "123")
            extra.unlink()
            for name in (archive, *installers):
                with self.subTest(name=name):
                    checksum = directory / (name + ".sha256")
                    saved = checksum.read_text()
                    checksum.write_text("changed")
                    with self.assertRaises(ValueError):
                        ci.verify(directory, "0.1.0-beta.4", target, "123")
                    checksum.unlink()
                    with self.assertRaises(ValueError):
                        ci.verify(directory, "0.1.0-beta.4", target, "123")
                    checksum.write_text(saved)
                    ci.verify(directory, "0.1.0-beta.4", target, "123")
            # A package without one of its installers is not promotable.
            (directory / installers[1]).unlink()
            with self.assertRaises(ValueError):
                ci.verify(directory, "0.1.0-beta.4", target, "123")

    def test_release_notes_list_every_installer_with_its_checksum(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            for target in ci.TARGETS:
                self.package(directory, target)
            table = ci.installer_sums(directory, "0.1.0-beta.4")
            rows = table.splitlines()[2:]
            self.assertEqual(len(rows), sum(len(v) for v in ci.INSTALLERS.values()))
            dmg = "Baylee-0.1.0-beta.4-aarch64.dmg"
            self.assertIn(f"| `{dmg}` | `{ci.digest(directory / dmg)}` |", rows)
            (directory / (dmg + ".sha256")).write_text("0" * 64 + "  other.dmg\n")
            with self.assertRaises(ValueError):
                ci.installer_sums(directory, "0.1.0-beta.4")
            (directory / (dmg + ".sha256")).unlink()
            with self.assertRaises(ValueError):
                ci.installer_sums(directory, "0.1.0-beta.4")

    def test_paths_are_not_versions(self):
        for version in ["../evil", "1.2.3\nextra", "1.2.3/a", "$(id)"]:
            with self.assertRaises(ValueError):
                ci.names(version, "aarch64-apple-darwin")



ROOT = Path(__file__).parents[3]


class SeatBridgeShips(unittest.TestCase):
    """beta.6 shipped no `baylee-seat`, so no room offered a language model."""

    VERSION = "0.0.0-test.1"
    TARGET = "x86_64-unknown-linux-gnu"

    def pack(self, bins):
        with tempfile.TemporaryDirectory() as tmp:
            for name in bins:
                (Path(tmp) / name).write_text(name)
            return subprocess.run(
                ["bash", str(ROOT / "scripts/package-client.sh"), tmp, self.TARGET, self.VERSION],
                cwd=ROOT,
                capture_output=True,
                text=True,
            )

    def test_the_archive_carries_the_seat_bridge_beside_the_client(self):
        done = self.pack(["baylee-client", "baylee-launch", "baylee-seat"])
        self.assertEqual(done.returncode, 0, done.stderr)
        archive = ROOT / done.stdout.strip().splitlines()[-1]
        try:
            with tarfile.open(archive) as tar:
                names = set(tar.getnames())
        finally:
            archive.unlink()
            stage = ROOT / "target/package" / f"baylee-client-{self.VERSION}-{self.TARGET}"
            subprocess.run(["rm", "-rf", str(stage)], check=True)
        top = f"baylee-client-{self.VERSION}-{self.TARGET}"
        for f in ("baylee-client", "baylee-runtime", "baylee-seat"):
            self.assertIn(f"{top}/{f}", names)

    def test_packing_without_the_seat_bridge_is_refused(self):
        done = self.pack(["baylee-client", "baylee-launch"])
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("baylee-seat", done.stderr)

    def test_ci_builds_it_and_every_installer_check_needs_it(self):
        workflow = (ROOT / ".github/workflows/client-packages.yml").read_text()
        self.assertRegex(workflow, r"cargo build [^\n]*--bin baylee-seat")
        checks = (ROOT / "scripts/release/check-installers.sh").read_text()
        for needed in (
            'need "$mnt/Baylee.app/Contents/MacOS/baylee-seat"',
            "baylee-seat.exe",
            "./opt/baylee/baylee-seat$",
            "need /opt/baylee/baylee-seat",
        ):
            self.assertIn(needed, checks.replace("\\.", "."))
        script = (ROOT / "scripts/package-client.sh").read_text()
        self.assertIn('codesign --force --sign - "$app/MacOS/$seat"', script)

if __name__ == "__main__":
    unittest.main()
