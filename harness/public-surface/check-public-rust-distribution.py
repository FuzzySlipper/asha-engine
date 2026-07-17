#!/usr/bin/env python3
"""Prove the public Rust facades resolve from an exact Git revision."""
from __future__ import annotations

import copy
import json
import os
import pathlib
import re
import shutil
import subprocess
import tempfile
import tomllib
from typing import Any

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
PUBLIC_MANIFEST = REPO_ROOT / "harness/public-surface/rust-crates.json"
FIXTURE_ROOT = REPO_ROOT / "harness/fixtures/public-rust-git-consumer"
REVISION_PATTERN = re.compile(r"^[0-9a-f]{40}$")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def run(command: list[str], *, cwd: pathlib.Path, env: dict[str, str] | None = None) -> str:
    completed = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({' '.join(command)}):\n{completed.stdout}"
        )
    return completed.stdout


def validate_consumer_manifest(
    document: dict[str, Any],
    *,
    approved_crates: set[str],
    expected_repository: str,
    expected_revision: str,
    expected_version: str,
) -> None:
    dependencies = document.get("dependencies")
    require(isinstance(dependencies, dict) and dependencies, "consumer must declare dependencies")
    for crate_name, specification in dependencies.items():
        require(crate_name in approved_crates, f"consumer directly imports unapproved crate {crate_name}")
        require(isinstance(specification, dict), f"{crate_name} dependency must use a detailed specification")
        require("path" not in specification, f"{crate_name} must not use a path dependency")
        require(specification.get("git") == expected_repository, f"{crate_name} repository is not governed")
        revision = specification.get("rev")
        require(
            isinstance(revision, str) and REVISION_PATTERN.fullmatch(revision) is not None,
            f"{crate_name} must use an exact 40-character Git revision",
        )
        require(revision == expected_revision, f"{crate_name} revision is stale for this proof")
        require(specification.get("version") == expected_version, f"{crate_name} version pin is incompatible")


def prove_rejections(
    valid_document: dict[str, Any],
    *,
    approved_crates: set[str],
    repository: str,
    revision: str,
    version: str,
) -> None:
    mutations: list[tuple[str, dict[str, Any]]] = []

    private_path = copy.deepcopy(valid_document)
    private_path["dependencies"]["core-state"] = {"path": "../asha-engine/engine-rs/crates/state/core-state"}
    mutations.append(("private/path escape", private_path))

    missing_revision = copy.deepcopy(valid_document)
    missing_revision["dependencies"]["asha-gameplay-module-sdk"].pop("rev")
    mutations.append(("missing revision", missing_revision))

    stale_revision = copy.deepcopy(valid_document)
    stale_revision["dependencies"]["asha-gameplay-module-sdk"]["rev"] = "0" * 40
    mutations.append(("stale revision", stale_revision))

    incompatible_version = copy.deepcopy(valid_document)
    incompatible_version["dependencies"]["asha-gameplay-module-sdk"]["version"] = "^9"
    mutations.append(("incompatible version", incompatible_version))

    for label, mutation in mutations:
        try:
            validate_consumer_manifest(
                mutation,
                approved_crates=approved_crates,
                expected_repository=repository,
                expected_revision=revision,
                expected_version=version,
            )
        except RuntimeError:
            continue
        raise RuntimeError(f"consumer validation accepted {label}")


def main() -> None:
    public_manifest = json.loads(PUBLIC_MANIFEST.read_text())
    distribution = public_manifest["distribution"]
    approved_crates = {record["crate"] for record in public_manifest["crates"]}
    template = (FIXTURE_ROOT / "Cargo.toml.in").read_text()

    require("path =" not in template, "clean consumer fixture must not contain path dependencies")
    require("engine-rs/crates" not in template, "clean consumer fixture must not name private crates")

    with (
        tempfile.TemporaryDirectory(prefix="asha-public-rust-source-") as source_temporary,
        tempfile.TemporaryDirectory(prefix="asha-public-rust-consumer-") as consumer_temporary,
    ):
        source_checkout = pathlib.Path(source_temporary) / "git-source"
        consumer_root = pathlib.Path(consumer_temporary)
        consumer_checkout = consumer_root / "consumer"
        target_dir = consumer_root / "target"
        shutil.copytree(
            REPO_ROOT,
            source_checkout,
            symlinks=True,
            ignore=shutil.ignore_patterns(".git", "target", "node_modules", "dist", "coverage"),
            ignore_dangling_symlinks=True,
        )
        run(["git", "init", "--quiet"], cwd=source_checkout)
        run(["git", "config", "user.name", "ASHA CI"], cwd=source_checkout)
        run(["git", "config", "user.email", "asha-ci@example.invalid"], cwd=source_checkout)
        run(["git", "add", "--all"], cwd=source_checkout)
        run(["git", "commit", "--quiet", "-m", "public Rust distribution fixture"], cwd=source_checkout)
        revision = run(["git", "rev-parse", "HEAD"], cwd=source_checkout).strip()
        repository = source_checkout.as_uri()

        consumer_checkout.mkdir()
        shutil.copytree(FIXTURE_ROOT / "src", consumer_checkout / "src")
        rendered_manifest = template.replace("__ASHA_GIT_URL__", repository).replace(
            "__ASHA_GIT_REV__", revision
        )
        (consumer_checkout / "Cargo.toml").write_text(rendered_manifest)
        consumer_document = tomllib.loads(rendered_manifest)
        validate_consumer_manifest(
            consumer_document,
            approved_crates=approved_crates,
            expected_repository=repository,
            expected_revision=revision,
            expected_version=distribution["versionRequirement"],
        )
        prove_rejections(
            consumer_document,
            approved_crates=approved_crates,
            repository=repository,
            revision=revision,
            version=distribution["versionRequirement"],
        )

        environment = os.environ.copy()
        environment["CARGO_TARGET_DIR"] = str(target_dir)
        # Resolve the exact local Git source and provision every registry
        # package before enforcing the offline consumer build. A fresh runner
        # must not rely on crates left in an ambient Cargo cache.
        run(["cargo", "generate-lockfile"], cwd=consumer_checkout, env=environment)
        run(["cargo", "fetch", "--locked"], cwd=consumer_checkout, env=environment)
        run(["cargo", "check", "--locked", "--offline"], cwd=consumer_checkout, env=environment)

        lock_text = (consumer_checkout / "Cargo.lock").read_text()
        for crate_name in ("asha-gameplay-module-sdk", "asha-runtime-session-composition"):
            require(f'name = "{crate_name}"' in lock_text, f"lockfile omitted {crate_name}")
        require(repository in lock_text, "lockfile omitted the exact local Git source")

    print("Public Rust Git distribution: OK")


if __name__ == "__main__":
    main()
