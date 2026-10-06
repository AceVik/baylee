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

# What scripts/package-installers.sh writes beside each archive. Players
# install from these; the updater only ever fetches the signed archive.
INSTALLERS = {
    "x86_64-unknown-linux-gnu": (
        "Baylee-{version}-x86_64.AppImage",
        "baylee_{version}_amd64.deb",
    ),
    "aarch64-unknown-linux-gnu": (
        "Baylee-{version}-aarch64.AppImage",
        "baylee_{version}_arm64.deb",
    ),
    "x86_64-pc-windows-msvc": ("Baylee-Setup-{version}-x64.exe",),
    "aarch64-pc-windows-msvc": ("Baylee-Setup-{version}-arm64.exe",),
    "aarch64-apple-darwin": ("Baylee-{version}-aarch64.dmg",),
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
    # This workflow has five packages (an archive and its installers each);
    # other CI artifacts may coexist.
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
    """The archive, its installers and the manifest of one target's package."""
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+[-+a-zA-Z0-9.]*", version):
        raise ValueError("invalid package version")
    archive = f"baylee-client-{version}-{target}.{TARGETS[target]}"
    installers = tuple(name.format(version=version) for name in INSTALLERS[target])
    return archive, installers, f"{target}.manifest.json"


def digest(path):
    with path.open("rb") as stream:
        value = hashlib.sha256()
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
        return value.hexdigest()


def identity(version, target, run):
    return {
        "schema": 2,
        "repository": os.environ["GITHUB_REPOSITORY"],
        "commit": os.environ["GITHUB_SHA"],
        "version": version,
        "target": target,
        "run_id": str(run),
    }


def checksum_line(sha, name):
    return f"{sha}  {name}\n"


def recorded(archive, installers, sums):
    """The manifest's file fields: the archive's digest and each installer's."""
    return {
        "archive": archive,
        "sha256": sums[archive],
        "installers": {name: sums[name] for name in installers},
    }


def write(directory, version, target):
    directory = Path(directory)
    archive, installers, manifest = names(version, target)
    sums = {name: digest(directory / name) for name in (archive, *installers)}
    for name, sha in sums.items():
        (directory / (name + ".sha256")).write_text(checksum_line(sha, name))
    data = identity(version, target, os.environ["GITHUB_RUN_ID"])
    data.update(recorded(archive, installers, sums))
    (directory / manifest).write_text(json.dumps(data, sort_keys=True) + "\n")


def verify(directory, version, target, run):
    directory = Path(directory)
    archive, installers, manifest = names(version, target)
    files = (archive, *installers)
    expected_files = {manifest, *files, *(name + ".sha256" for name in files)}
    if {p.name for p in directory.iterdir()} != expected_files:
        raise ValueError("unexpected or missing package files")
    if any(p.is_symlink() or not p.is_file() for p in directory.iterdir()):
        raise ValueError("package must contain regular files only")
    data = json.loads((directory / manifest).read_text())
    sums = {name: digest(directory / name) for name in files}
    expected = identity(version, target, run)
    expected.update(recorded(archive, installers, sums))
    if data != expected:
        raise ValueError("package provenance or checksum mismatch")
    for name, sha in sums.items():
        if (directory / (name + ".sha256")).read_text() != checksum_line(sha, name):
            raise ValueError(f"checksum file mismatch: {name}")


def installer_sums(directory, version):
    """The release notes' table of installers and their SHA-256, read from
    the promoted `.sha256` files; every installer of every target must be
    there, each file naming itself."""
    directory = Path(directory)
    lines = ["| Installer | SHA-256 |", "| --- | --- |"]
    for target in TARGETS:
        for name in names(version, target)[1]:
            path = directory / (name + ".sha256")
            if not (directory / name).is_file() or not path.is_file():
                raise ValueError(f"missing installer or checksum: {name}")
            sha, _, named = path.read_text().rstrip("\n").partition("  ")
            if named != name or not re.fullmatch(r"[0-9a-f]{64}", sha):
                raise ValueError(f"malformed checksum file: {path.name}")
            lines.append(f"| `{name}` | `{sha}` |")
    return "\n".join(lines)


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
    elif command == "installer-sums" and len(values) == 2:
        print(installer_sums(*values))
    else:
        raise ValueError(
            "usage: ci_artifacts.py resolve|verify-run ID|write DIR VERSION TARGET"
            "|verify DIR VERSION TARGET RUN|installer-sums DIR VERSION"
        )


if __name__ == "__main__":
    try:
        main(sys.argv[1:])
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        sys.exit(f"::error::{error}")
