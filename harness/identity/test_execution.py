#!/usr/bin/env python3
"""Negative and attribution tests for proof identity scheduling."""

from __future__ import annotations

import json
import os
import pathlib
import tempfile
import unittest

import execution


class ProofExecutionTests(unittest.TestCase):
    def test_execution_identity_collision_is_rejected(self) -> None:
        with self.assertRaisesRegex(execution.ExecutionError, "identity collision"):
            execution.definition_index([{"id": "same"}, {"id": "same"}])

    def test_stale_cache_receipt_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            paths = execution.cache_paths(root, "sha256:current")
            paths["directory"].mkdir(parents=True)
            paths["stdout"].write_text("out", encoding="utf-8")
            paths["stderr"].write_text("err", encoding="utf-8")
            paths["receipt"].write_text(
                json.dumps({"fingerprint": "sha256:stale", "exitCode": 0}),
                encoding="utf-8",
            )
            self.assertFalse(execution.cache_valid(paths, "sha256:current"))

    def test_missing_provider_is_rejected(self) -> None:
        catalog = {"families": {"providers": [{"id": "known"}]}}
        with self.assertRaisesRegex(execution.ExecutionError, "missing provider"):
            execution.provider_digest(catalog, ["unknown"])

    def test_divergent_environment_changes_fingerprint(self) -> None:
        definition = {"id": "proof", "command": ["cargo", "test"], "providerIds": []}
        settings = {"environmentKeys": ["CI"], "environmentPrefixes": []}
        catalog = {"families": {"providers": []}}
        first, _ = execution.execution_fingerprint(
            definition, settings, catalog, {"CI": "first"}, {"cargo": "same"}, "sha256:inputs"
        )
        second, _ = execution.execution_fingerprint(
            definition, settings, catalog, {"CI": "second"}, {"cargo": "same"}, "sha256:inputs"
        )
        self.assertNotEqual(first, second)

    def test_reviewed_build_environment_changes_fingerprint(self) -> None:
        definition = {"id": "proof", "command": ["cargo", "test"], "providerIds": []}
        settings = execution.load_json(execution.DEFINITIONS)
        catalog = {"families": {"providers": []}}
        baseline_environment = {
            "CC": "gcc",
            "CFLAGS": "-O1",
            "CPATH": "/opt/headers-one",
            "C_INCLUDE_PATH": "/opt/c-headers-one",
            "HOME": "/home/proof-one",
            "MACOSX_DEPLOYMENT_TARGET": "13.0",
            "MAKEFLAGS": "-j2",
            "NODE_ENV": "development",
            "TMPDIR": "/tmp/proof-one",
        }
        baseline, inputs = execution.execution_fingerprint(
            definition,
            settings,
            catalog,
            baseline_environment,
            {"cargo": "same"},
            "sha256:inputs",
        )
        for key, value in baseline_environment.items():
            self.assertEqual(inputs["environment"][key], value)

        for key, changed_value in (
            ("CC", "clang"),
            ("CFLAGS", "-O2"),
            ("CPATH", "/opt/headers-two"),
            ("C_INCLUDE_PATH", "/opt/c-headers-two"),
            ("HOME", "/home/proof-two"),
            ("MACOSX_DEPLOYMENT_TARGET", "14.0"),
            ("MAKEFLAGS", "-j8"),
            ("NODE_ENV", "production"),
            ("TMPDIR", "/tmp/proof-two"),
        ):
            changed_environment = {**baseline_environment, key: changed_value}
            changed, _ = execution.execution_fingerprint(
                definition,
                settings,
                catalog,
                changed_environment,
                {"cargo": "same"},
                "sha256:inputs",
            )
            with self.subTest(key=key):
                self.assertNotEqual(baseline, changed)

    def test_cargo_toolchain_tracks_configured_external_tools_and_libclang(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            tool = root / "configured-tool"
            tool.write_text("#!/bin/sh\necho configured-tool-one\n", encoding="utf-8")
            tool.chmod(0o755)
            libclang = root / "libclang.so"
            libclang.write_bytes(b"libclang-one")
            environment = {
                **os.environ,
                "AR": str(tool),
                "CC": str(tool),
                "CXX": str(tool),
                "LD": str(tool),
                "RANLIB": str(tool),
                "RUSTC_LINKER": str(tool),
                "CLANG_PATH": str(tool),
                "LLVM_CONFIG_PATH": str(tool),
                "PKG_CONFIG": str(tool),
                "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER": str(tool),
                "LIBCLANG_PATH": str(libclang),
                "HOME": str(root / "home"),
                "CARGO_HOME": str(root / "cargo-home"),
            }
            cargo_home = pathlib.Path(environment["CARGO_HOME"])
            cargo_home.mkdir(parents=True)
            cargo_config = cargo_home / "config.toml"
            cargo_config.write_text(
                "[target.x86_64-unknown-linux-gnu]\n"
                f'linker = "{tool}"\n',
                encoding="utf-8",
            )
            first = execution.toolchain_digest(["cargo", "test"], environment)
            for key in (
                "external:ar",
                "external:cc",
                "external:cxx",
                "external:linker",
                "external:ranlib",
                "external:CLANG_PATH",
                "external:LLVM_CONFIG_PATH",
                "external:pkg-config",
                "external:CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
                "external:LIBCLANG_PATH",
                f"cargo-config:{cargo_config}:target.x86_64-unknown-linux-gnu.linker",
            ):
                with self.subTest(key=key):
                    self.assertIn(key, first)

            tool.write_text("#!/bin/sh\necho configured-tool-two\n", encoding="utf-8")
            tool.chmod(0o755)
            libclang.write_bytes(b"libclang-two")
            cargo_config.write_text(
                "[target.x86_64-unknown-linux-gnu]\n"
                f'linker = "{tool}"\n'
                'rustflags = ["-C", "target-cpu=native"]\n',
                encoding="utf-8",
            )
            second = execution.toolchain_digest(["cargo", "test"], environment)
            self.assertNotEqual(first["external:cc"], second["external:cc"])
            self.assertNotEqual(
                first["external:LIBCLANG_PATH"],
                second["external:LIBCLANG_PATH"],
            )
            self.assertNotEqual(
                first["cargo-configuration"],
                second["cargo-configuration"],
            )

    def test_pkg_config_executable_changes_cargo_and_shell_toolchains(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            first_tool = root / "pkg-config-one"
            second_tool = root / "pkg-config-two"
            for path, label in ((first_tool, "one"), (second_tool, "two")):
                path.write_text(f"#!/bin/sh\necho pkg-config-{label}\n", encoding="utf-8")
                path.chmod(0o755)
            baseline_environment = {**os.environ, "PKG_CONFIG": str(first_tool)}
            changed_environment = {**os.environ, "PKG_CONFIG": str(second_tool)}

            for command in (["cargo", "test"], ["harness/ci/check-rust.sh"]):
                first = execution.toolchain_digest(command, baseline_environment)
                second = execution.toolchain_digest(command, changed_environment)
                with self.subTest(command=command[0]):
                    self.assertIn("external:pkg-config", first)
                    self.assertNotEqual(
                        first["external:pkg-config"],
                        second["external:pkg-config"],
                    )

    def test_pnpm_toolchain_tracks_same_path_user_configuration_content(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            user_config = root / "user.npmrc"
            user_config.write_text("registry=https://registry-one.invalid\n", encoding="utf-8")
            environment = {
                **os.environ,
                "HOME": str(root),
                "NPM_CONFIG_USERCONFIG": str(user_config),
            }
            command = ["pnpm", "--dir", "ts", "--filter", "@asha/app", "test"]
            first = execution.toolchain_digest(command, environment)
            self.assertIn("npm-configuration", first)
            self.assertIn(user_config.as_posix(), first["npm-configuration"])

            user_config.write_text("registry=https://registry-two.invalid\n", encoding="utf-8")
            second = execution.toolchain_digest(command, environment)
            self.assertNotEqual(first["npm-configuration"], second["npm-configuration"])

    def test_command_input_toolchain_and_provider_changes_invalidate_fingerprint(self) -> None:
        definition = {"id": "proof", "command": ["cargo", "test"], "providerIds": ["provider"]}
        settings = {"environmentKeys": [], "environmentPrefixes": []}
        catalog = {"families": {"providers": [{"id": "provider", "sourceHash": "sha256:first"}]}}
        baseline, _ = execution.execution_fingerprint(
            definition, settings, catalog, {}, {"cargo": "first"}, "sha256:fixture-and-generated-contract"
        )
        mutations = [
            ({**definition, "command": ["cargo", "test", "--lib"]}, catalog, {"cargo": "first"}, "sha256:fixture-and-generated-contract"),
            (definition, catalog, {"cargo": "first"}, "sha256:changed-fixture-or-generated-contract"),
            (definition, catalog, {"cargo": "second"}, "sha256:fixture-and-generated-contract"),
            (definition, {"families": {"providers": [{"id": "provider", "sourceHash": "sha256:second"}]}}, {"cargo": "first"}, "sha256:fixture-and-generated-contract"),
        ]
        for changed_definition, changed_catalog, changed_toolchain, changed_inputs in mutations:
            fingerprint, _ = execution.execution_fingerprint(
                changed_definition,
                settings,
                changed_catalog,
                {},
                changed_toolchain,
                changed_inputs,
            )
            self.assertNotEqual(baseline, fingerprint)

    def test_shared_execution_retains_every_attribution(self) -> None:
        shared = {
            "fingerprint": "sha256:same",
            "fingerprintInputs": {"normalizedCommand": ["cargo", "test"]},
            "command": ["cargo", "test"],
            "executionIds": ["proof.one"],
            "artifactIds": ["evidence.one"],
            "attributions": [{"suiteId": "suite.one", "probeIds": ["probe.one"], "assertionIds": ["assertion.one"]}],
        }
        other = {
            **shared,
            "executionIds": ["proof.two"],
            "artifactIds": ["evidence.two"],
            "attributions": [{"suiteId": "suite.two", "probeIds": ["probe.two"], "assertionIds": ["assertion.two"]}],
        }
        grouped = execution.group_equivalent([shared, other])
        self.assertEqual(len(grouped), 1)
        self.assertEqual(grouped[0]["executionIds"], ["proof.one", "proof.two"])
        self.assertEqual(
            [item["suiteId"] for item in grouped[0]["attributions"]],
            ["suite.one", "suite.two"],
        )


if __name__ == "__main__":
    unittest.main()
