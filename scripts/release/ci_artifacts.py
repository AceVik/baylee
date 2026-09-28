#!/usr/bin/env python3
"""Promote only same-commit, successful main CI packages; never PR artifacts."""

import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

TARGETS = {
    "x86_64-unknown-linux-gnu": "tar.gz",
    "aarch64-unknown-linux-gnu": "tar.gz",
    "x86_64-pc-windows-msvc": "zip",
    "aarch64-pc-windows-msvc": "zip",
    "aarch64-apple-darwin": "zip",
}


def api(path):
    return json.loads(subprocess.check_output(["gh", "api", path], text=True))


def trusted_run(run, repository, sha):
    return (
        run.get("head_sha") == sha
        and run.get("head_branch") == "main"
        and run.get("event") == "push"
        and run.get("status") == "completed"
        and run.get("conclusion") == "success"
        and run.get("path") == ".github/workflows/ci.yml"
        and run.get("repository", {}).get("full_name") == repository
        and run.get("head_repository", {}).get("full_name") == repository
    )


def verify_run(run_id):
    if not str(run_id).isdigit():
        raise ValueError("invalid CI run ID")
    repo, sha = os.environ["GITHUB_REPOSITORY"], os.environ["GITHUB_SHA"]
    run = api(f"repos/{repo}/actions/runs/{run_id}")
    if not trusted_run(run, repo, sha):
        raise ValueError(
            "source must be a successful main push CI run for this repository and commit"
        )
    # This workflow has five packages; other CI artifacts may coexist.
    artifacts = []
    page = 1
    while True:
        batch = api(
            f"repos/{repo}/actions/runs/{run_id}/artifacts?per_page=100&page={page}"
        )["artifacts"]
        artifacts.extend(batch)
        if len(batch) < 100:
            break
        page += 1
    for target in TARGETS:
        matches = [a for a in artifacts if a["name"] == f"client-{target}"]
        if len(matches) != 1 or matches[0].get("expired", True):
            raise ValueError(
                f"missing/expired/ambiguous {target} package; rerun CI on main at this commit"
            )


def resolve():
    repo, sha = os.environ["GITHUB_REPOSITORY"], os.environ["GITHUB_SHA"]
    runs = api(
        f"repos/{repo}/actions/workflows/ci.yml/runs?head_sha={sha}&event=push&branch=main&status=success&per_page=100"
    )["workflow_runs"]
    # Prefer the newest successful build; never silently use a different SHA.
    for run in runs:
        if trusted_run(run, repo, sha):
            verify_run(run["id"])
            return str(run["id"])
    raise ValueError(
        "no successful main CI at this commit; push and wait for CI before tagging"
    )


def names(version, target):
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+[-+a-zA-Z0-9.]*", version):
        raise ValueError("invalid package version")
    archive = f"baylee-client-{version}-{target}.{TARGETS[target]}"
    return archive, f"{target}.manifest.json"


def digest(path):
    with path.open("rb") as stream:
        value = hashlib.sha256()
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
        return value.hexdigest()


def identity(version, target, run):
    return {
        "schema": 1,
        "repository": os.environ["GITHUB_REPOSITORY"],
        "commit": os.environ["GITHUB_SHA"],
        "version": version,
        "target": target,
        "run_id": str(run),
    }


def write(directory, version, target):
    directory = Path(directory)
    archive, manifest = names(version, target)
    checksum = digest(directory / archive)
    (directory / (archive + ".sha256")).write_text(f"{checksum}  {archive}\n")
    data = identity(version, target, os.environ["GITHUB_RUN_ID"])
    data.update(archive=archive, sha256=checksum)
    (directory / manifest).write_text(json.dumps(data, sort_keys=True) + "\n")


def verify(directory, version, target, run):
    directory = Path(directory)
    archive, manifest = names(version, target)
    expected_files = {archive, archive + ".sha256", manifest}
    if {p.name for p in directory.iterdir()} != expected_files:
        raise ValueError("unexpected or missing package files")
    if any(p.is_symlink() or not p.is_file() for p in directory.iterdir()):
        raise ValueError("package must contain regular files only")
    data = json.loads((directory / manifest).read_text())
    expected = identity(version, target, run)
    expected.update(archive=archive, sha256=digest(directory / archive))
    if data != expected:
        raise ValueError("package provenance or checksum mismatch")
    if (
        directory / (archive + ".sha256")
    ).read_text() != f"{expected['sha256']}  {archive}\n":
        raise ValueError("checksum file mismatch")


def main(args):
    command, *values = args
    if command == "resolve" and not values:
        print(resolve())
    elif command == "verify-run" and len(values) == 1:
        verify_run(*values)
    elif command == "write" and len(values) == 3:
        write(*values)
    elif command == "verify" and len(values) == 4:
        verify(*values)
    else:
        raise ValueError(
            "usage: ci_artifacts.py resolve|verify-run ID|write DIR VERSION TARGET|verify DIR VERSION TARGET RUN"
        )


if __name__ == "__main__":
    try:
        main(sys.argv[1:])
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        sys.exit(f"::error::{error}")
