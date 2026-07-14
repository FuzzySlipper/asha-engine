#!/usr/bin/env python3
"""Run equivalent proof commands once and retain every consuming attribution."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
import shlex
import shutil
import subprocess
import sys
import tomllib
from typing import Any, Iterable

ROOT = pathlib.Path(__file__).resolve().parents[2]
CACHE_ROOT = ROOT / "harness/smoke-out/proof-execution"
DEFINITIONS = ROOT / "harness/identity/executions.json"
CATALOG = ROOT / "harness/identity/catalog.json"
CONFORMANCE = ROOT / "harness/conformance/probe-inventory.json"
IGNORED_PARTS = {".git", "node_modules", "target", "smoke-out", "__pycache__"}

# Downstream Cargo fixtures share a disposable build root outside their source
# workspaces. The effective value is part of the proof-execution fingerprint via
# the existing CARGO_ environment-prefix rule.
os.environ.setdefault("CARGO_TARGET_DIR", str(ROOT / "target" / "proof-execution"))


class ExecutionError(ValueError):
    """An execution definition is ambiguous or cannot be resolved."""


def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def stable_hash(value: Any) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def definition_index(definitions: Iterable[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for definition in definitions:
        identity = definition.get("id")
        if not isinstance(identity, str) or not identity:
            raise ExecutionError("execution definition has a missing id")
        if identity in result:
            raise ExecutionError(f"execution identity collision: {identity}")
        result[identity] = definition
    return result


def files_for_input(path: pathlib.Path) -> list[pathlib.Path]:
    if not path.exists():
        raise ExecutionError(f"execution input does not exist: {path}")
    if path.is_file():
        return [path]
    return sorted(
        candidate
        for candidate in path.rglob("*")
        if candidate.is_file() and not any(part in IGNORED_PARTS for part in candidate.parts)
    )


def input_digest(paths: Iterable[str]) -> str:
    entries: list[dict[str, str]] = []
    for source in sorted(set(paths)):
        path = (ROOT / source).resolve()
        for candidate in files_for_input(path):
            try:
                label = candidate.relative_to(ROOT).as_posix()
            except ValueError:
                label = candidate.as_posix()
            entries.append({
                "path": label,
                "hash": "sha256:" + hashlib.sha256(candidate.read_bytes()).hexdigest(),
            })
    return stable_hash(entries)


def selected_environment(
    settings: dict[str, Any], environment: dict[str, str] | None = None
) -> dict[str, str]:
    values = dict(os.environ if environment is None else environment)
    keys = set(settings.get("environmentKeys", []))
    prefixes = tuple(settings.get("environmentPrefixes", []))
    keys.update(key for key in values if prefixes and key.startswith(prefixes))
    return {key: values.get(key, "<unset>") for key in sorted(keys)}


def executable_identity(
    command_value: str,
    environment: dict[str, str],
    *,
    required: bool,
) -> str:
    try:
        command = shlex.split(command_value)
    except ValueError as error:
        raise ExecutionError(f"invalid configured tool command {command_value!r}: {error}") from error
    if not command:
        raise ExecutionError("configured tool command is empty")
    resolved = shutil.which(command[0], path=environment.get("PATH"))
    if resolved is None:
        if required:
            raise ExecutionError(f"toolchain executable is unavailable: {command[0]}")
        return json.dumps(
            {"command": command, "status": "unavailable"},
            sort_keys=True,
            separators=(",", ":"),
        )

    resolved_path = pathlib.Path(resolved).resolve()
    version_attempts: list[dict[str, Any]] = []
    version_output: str | None = None
    for version_argument in ("--version", "-v"):
        completed = subprocess.run(
            [*command, version_argument],
            cwd=ROOT,
            check=False,
            text=True,
            capture_output=True,
            env=environment,
        )
        output = "\n".join(
            part.strip() for part in (completed.stdout, completed.stderr) if part.strip()
        )[:16_384]
        version_attempts.append({
            "argument": version_argument,
            "exitCode": completed.returncode,
            "output": output,
        })
        if completed.returncode == 0:
            version_output = output
            break
    if version_output is None and required:
        raise ExecutionError(f"toolchain probe failed: {command_value}")

    executable_hash = "<not-a-file>"
    if resolved_path.is_file():
        executable_hash = "sha256:" + hashlib.sha256(resolved_path.read_bytes()).hexdigest()
    return json.dumps(
        {
            "command": command,
            "resolvedExecutable": resolved_path.as_posix(),
            "executableHash": executable_hash,
            "version": version_output,
            "versionAttempts": version_attempts,
        },
        sort_keys=True,
        separators=(",", ":"),
    )


def configured_path_identity(value: str) -> str:
    path = pathlib.Path(value).expanduser().resolve()
    if not path.exists():
        return json.dumps(
            {"configuredPath": path.as_posix(), "status": "unavailable"},
            sort_keys=True,
            separators=(",", ":"),
        )
    candidates = [path] if path.is_file() else sorted(
        candidate
        for candidate in path.glob("libclang*")
        if candidate.is_file()
    )
    entries = [
        {
            "path": candidate.as_posix(),
            "hash": "sha256:" + hashlib.sha256(candidate.read_bytes()).hexdigest(),
        }
        for candidate in candidates
    ]
    return json.dumps(
        {
            "configuredPath": path.as_posix(),
            "entries": entries,
            "status": "available",
        },
        sort_keys=True,
        separators=(",", ":"),
    )


def cargo_configuration_paths(environment: dict[str, str]) -> list[pathlib.Path]:
    candidates: list[pathlib.Path] = []
    for directory in (ROOT, *ROOT.parents):
        candidates.extend((directory / ".cargo/config.toml", directory / ".cargo/config"))
    cargo_home = environment.get("CARGO_HOME")
    if cargo_home is None:
        home = environment.get("HOME")
        cargo_home = str(pathlib.Path(home) / ".cargo") if home else None
    if cargo_home is not None:
        directory = pathlib.Path(cargo_home).expanduser()
        candidates.extend((directory / "config.toml", directory / "config"))
    unique: dict[str, pathlib.Path] = {}
    for candidate in candidates:
        resolved = candidate.resolve()
        if resolved.is_file():
            unique[resolved.as_posix()] = resolved
    return [unique[key] for key in sorted(unique)]


def configuration_file_identity(paths: list[pathlib.Path]) -> str:
    return json.dumps(
        [
            {
                "path": path.as_posix(),
                "hash": "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest(),
            }
            for path in paths
        ],
        sort_keys=True,
        separators=(",", ":"),
    )


def npm_configuration_paths(
    command: list[str], environment: dict[str, str]
) -> list[pathlib.Path]:
    candidates = [ROOT / ".npmrc"]
    for index, argument in enumerate(command[:-1]):
        if argument not in ("--dir", "--cwd", "-C"):
            continue
        command_directory = pathlib.Path(command[index + 1]).expanduser()
        if not command_directory.is_absolute():
            command_directory = ROOT / command_directory
        candidates.append(command_directory / ".npmrc")

    explicit_user_config = environment.get("NPM_CONFIG_USERCONFIG")
    if explicit_user_config:
        candidates.append(pathlib.Path(explicit_user_config).expanduser())
    else:
        home = environment.get("HOME")
        if home:
            candidates.append(pathlib.Path(home).expanduser() / ".npmrc")

    explicit_global_config = environment.get("NPM_CONFIG_GLOBALCONFIG")
    if explicit_global_config:
        candidates.append(pathlib.Path(explicit_global_config).expanduser())
    prefix = environment.get("NPM_CONFIG_PREFIX")
    if prefix:
        candidates.append(pathlib.Path(prefix).expanduser() / "etc/npmrc")

    unique: dict[str, pathlib.Path] = {}
    for candidate in candidates:
        resolved = candidate.resolve()
        if resolved.is_file():
            unique[resolved.as_posix()] = resolved
    return [unique[key] for key in sorted(unique)]


def cargo_configuration_tool_commands(paths: list[pathlib.Path]) -> dict[str, str]:
    result: dict[str, str] = {}
    for path in paths:
        try:
            document = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as error:
            raise ExecutionError(f"cannot read Cargo configuration {path}: {error}") from error
        build = document.get("build", {})
        if isinstance(build, dict):
            for key in ("rustc", "rustc-wrapper", "rustc-workspace-wrapper"):
                value = build.get(key)
                if isinstance(value, str) and value:
                    result[f"cargo-config:{path}:{key}"] = value
        target = document.get("target", {})
        if isinstance(target, dict):
            for target_name, settings in target.items():
                if not isinstance(settings, dict):
                    continue
                for key in ("ar", "linker"):
                    value = settings.get(key)
                    if isinstance(value, str) and value:
                        result[f"cargo-config:{path}:target.{target_name}.{key}"] = value
        configured_environment = document.get("env", {})
        if isinstance(configured_environment, dict):
            for key in ("AR", "CC", "CLANG_PATH", "CXX", "LD", "LLVM_CONFIG_PATH", "RANLIB", "RUSTC_LINKER"):
                value = configured_environment.get(key)
                if isinstance(value, dict):
                    value = value.get("value")
                if isinstance(value, str) and value:
                    result[f"cargo-config:{path}:env.{key}"] = value
    return result


def cargo_external_tool_commands(environment: dict[str, str]) -> dict[str, tuple[str, bool]]:
    configured: dict[str, tuple[str, bool]] = {
        "external:cc": (environment.get("CC", "cc"), "CC" in environment),
        "external:cxx": (environment.get("CXX", "c++"), "CXX" in environment),
        "external:ar": (environment.get("AR", "ar"), "AR" in environment),
        "external:ranlib": (environment.get("RANLIB", "ranlib"), "RANLIB" in environment),
        "external:pkg-config": (
            environment.get("PKG_CONFIG", "pkg-config"),
            "PKG_CONFIG" in environment,
        ),
        "external:linker": (
            environment.get("RUSTC_LINKER", environment.get("LD", environment.get("CC", "cc"))),
            any(key in environment for key in ("RUSTC_LINKER", "LD", "CC")),
        ),
    }
    optional_keys = (
        "CLANG_PATH",
        "LLVM_CONFIG_PATH",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
    )
    for key in optional_keys:
        value = environment.get(key)
        if value:
            configured[f"external:{key}"] = (value, True)
    target_tool = re.compile(
        r"^(?:AR|CC|CXX|RANLIB)_.+$"
        r"|^(?:HOST|TARGET)_(?:AR|CC|CXX|RANLIB)$"
        r"|^CARGO_TARGET_.+_(?:AR|LINKER)$"
    )
    for key, value in environment.items():
        if value and target_tool.fullmatch(key):
            configured[f"external:{key}"] = (value, True)
    return configured


def toolchain_digest(
    command: list[str], environment: dict[str, str] | None = None
) -> dict[str, str]:
    effective_environment = dict(os.environ if environment is None else environment)
    executable = command[0]
    if executable == "cargo":
        probes = {"cargo": "cargo", "rustc": effective_environment.get("RUSTC", "rustc")}
    elif executable == "pnpm":
        probes = {"pnpm": "pnpm", "node": "node"}
    else:
        probes = {"bash": "bash", "cargo": "cargo", "rustc": "rustc", "pnpm": "pnpm", "node": "node"}
    result: dict[str, str] = {}
    for label, probe in probes.items():
        result[label] = executable_identity(probe, effective_environment, required=True)
    if executable == "cargo":
        configuration_paths = cargo_configuration_paths(effective_environment)
        result["cargo-configuration"] = configuration_file_identity(configuration_paths)
        configured_tools = cargo_external_tool_commands(effective_environment)
        for label, value in cargo_configuration_tool_commands(configuration_paths).items():
            configured_tools[label] = (value, True)
        for label, (value, explicitly_configured) in sorted(configured_tools.items()):
            result[label] = executable_identity(
                value,
                effective_environment,
                required=explicitly_configured,
            )
        libclang_path = effective_environment.get("LIBCLANG_PATH")
        if libclang_path:
            result["external:LIBCLANG_PATH"] = configured_path_identity(libclang_path)
    elif executable == "pnpm":
        result["npm-configuration"] = configuration_file_identity(
            npm_configuration_paths(command, effective_environment)
        )
    else:
        result["external:pkg-config"] = executable_identity(
            effective_environment.get("PKG_CONFIG", "pkg-config"),
            effective_environment,
            required="PKG_CONFIG" in effective_environment,
        )
    return result


def provider_digest(catalog: dict[str, Any], provider_ids: list[str]) -> str:
    providers = {
        item["id"]: item for item in catalog["families"]["providers"]
    }
    missing = sorted(set(provider_ids) - set(providers))
    if missing:
        raise ExecutionError(f"execution references missing provider identities: {missing}")
    return stable_hash([providers[identity] for identity in sorted(provider_ids)])


def execution_fingerprint(
    definition: dict[str, Any],
    settings: dict[str, Any],
    catalog: dict[str, Any],
    environment: dict[str, str],
    toolchain: dict[str, str],
    inputs_hash: str,
) -> tuple[str, dict[str, Any]]:
    command = definition.get("command")
    if not isinstance(command, list) or not command or not all(isinstance(item, str) and item for item in command):
        raise ExecutionError(f"execution {definition.get('id')} has an invalid command")
    payload = {
        "normalizedCommand": command,
        "environment": selected_environment(settings, environment),
        "inputDigest": inputs_hash,
        "providerDigest": provider_digest(catalog, definition.get("providerIds", [])),
        "toolchain": toolchain,
    }
    return stable_hash(payload), payload


def suite_attributions(document: dict[str, Any]) -> dict[str, list[dict[str, Any]]]:
    probes_by_suite: dict[str, list[dict[str, Any]]] = {}
    for probe in document["semanticProbes"]:
        probes_by_suite.setdefault(probe["suite"], []).append(probe)
    result: dict[str, list[dict[str, Any]]] = {}
    for suite in document["suites"]:
        assertions = sorted(
            evidence["assertionId"]
            for probe in probes_by_suite.get(suite["id"], [])
            for evidence in probe["evidence"]
        )
        result.setdefault(suite["executionId"], []).append({
            "suiteId": suite["id"],
            "probeIds": sorted(probe["id"] for probe in probes_by_suite.get(suite["id"], [])),
            "assertionIds": assertions,
        })
    return result


def make_plan(
    execution_ids: list[str] | None = None,
    extra_attributions: list[str] | None = None,
    environment: dict[str, str] | None = None,
) -> list[dict[str, Any]]:
    settings = load_json(DEFINITIONS)
    definitions = definition_index(settings["executions"])
    catalog = load_json(CATALOG)
    conformance = load_json(CONFORMANCE)
    attributions = suite_attributions(conformance)
    selected_ids = sorted(definitions) if execution_ids is None else execution_ids
    unknown = sorted(set(selected_ids) - set(definitions))
    if unknown:
        raise ExecutionError(f"unknown execution identities: {unknown}")
    common_inputs = settings.get("commonInputs", [])
    planned: list[dict[str, Any]] = []
    toolchains: dict[tuple[str, ...], dict[str, str]] = {}
    effective_environment = dict(os.environ if environment is None else environment)
    for identity in selected_ids:
        definition = definitions[identity]
        command = definition["command"]
        toolchain_key = tuple(command[:1])
        if toolchain_key not in toolchains:
            toolchains[toolchain_key] = toolchain_digest(command, effective_environment)
        toolchain = toolchains[toolchain_key]
        fingerprint, inputs = execution_fingerprint(
            definition,
            settings,
            catalog,
            effective_environment,
            toolchain,
            input_digest(common_inputs + definition.get("inputs", [])),
        )
        attribution = list(attributions.get(identity, []))
        attribution.extend(
            {"suiteId": item, "probeIds": [], "assertionIds": []}
            for item in (extra_attributions or [])
        )
        planned.append({
            "executionIds": [identity],
            "artifactIds": [definition["artifactId"]],
            "fingerprint": fingerprint,
            "fingerprintInputs": inputs,
            "command": command,
            "attributions": attribution,
        })
    return group_equivalent(planned)


def group_equivalent(planned: list[dict[str, Any]]) -> list[dict[str, Any]]:
    groups: dict[str, dict[str, Any]] = {}
    for item in planned:
        fingerprint = item["fingerprint"]
        if fingerprint not in groups:
            groups[fingerprint] = {
                **item,
                "executionIds": list(item["executionIds"]),
                "artifactIds": list(item["artifactIds"]),
                "attributions": list(item["attributions"]),
            }
            continue
        group = groups[fingerprint]
        if group["command"] != item["command"] or group["fingerprintInputs"] != item["fingerprintInputs"]:
            raise ExecutionError(f"fingerprint collision: {fingerprint}")
        group["executionIds"].extend(item["executionIds"])
        group["artifactIds"].extend(item["artifactIds"])
        group["attributions"].extend(item["attributions"])
    for group in groups.values():
        group["executionIds"] = sorted(set(group["executionIds"]))
        group["artifactIds"] = sorted(set(group["artifactIds"]))
        unique = {item["suiteId"]: item for item in group["attributions"]}
        group["attributions"] = [unique[key] for key in sorted(unique)]
    return sorted(groups.values(), key=lambda item: item["fingerprint"])


def cache_paths(cache_root: pathlib.Path, fingerprint: str) -> dict[str, pathlib.Path]:
    directory = cache_root / fingerprint.removeprefix("sha256:")
    return {
        "directory": directory,
        "receipt": directory / "receipt.json",
        "stdout": directory / "stdout.log",
        "stderr": directory / "stderr.log",
    }


def cache_valid(paths: dict[str, pathlib.Path], fingerprint: str) -> bool:
    if not all(paths[key].is_file() for key in ("receipt", "stdout", "stderr")):
        return False
    try:
        receipt = load_json(paths["receipt"])
    except (OSError, json.JSONDecodeError):
        return False
    return receipt.get("fingerprint") == fingerprint and receipt.get("exitCode") == 0


def merge_attributions(*groups: list[dict[str, Any]]) -> list[dict[str, Any]]:
    merged: dict[str, dict[str, Any]] = {}
    for group in groups:
        for item in group:
            identity = item["suiteId"]
            current = merged.setdefault(identity, {"suiteId": identity, "probeIds": [], "assertionIds": []})
            current["probeIds"] = sorted(set(current["probeIds"]) | set(item.get("probeIds", [])))
            current["assertionIds"] = sorted(
                set(current["assertionIds"]) | set(item.get("assertionIds", []))
            )
    return [merged[key] for key in sorted(merged)]


def run_plan(
    plan: list[dict[str, Any]], cache_root: pathlib.Path = CACHE_ROOT
) -> tuple[int, dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for item in plan:
        paths = cache_paths(cache_root, item["fingerprint"])
        paths["directory"].mkdir(parents=True, exist_ok=True)
        hit = cache_valid(paths, item["fingerprint"])
        prior_attributions: list[dict[str, Any]] = []
        if hit:
            prior_attributions = load_json(paths["receipt"]).get("attributions", [])
            exit_code = 0
        else:
            print(
                f"==> proof execution {', '.join(item['executionIds'])}: "
                f"{' '.join(item['command'])}",
                flush=True,
            )
            with paths["stdout"].open("w", encoding="utf-8") as stdout, paths["stderr"].open(
                "w", encoding="utf-8"
            ) as stderr:
                completed = subprocess.run(
                    item["command"], cwd=ROOT, check=False, text=True, stdout=stdout, stderr=stderr
                )
            exit_code = completed.returncode
        attributions = merge_attributions(prior_attributions, item["attributions"])
        receipt = {
            "schemaVersion": 1,
            "fingerprint": item["fingerprint"],
            "exitCode": exit_code,
            "executionIds": item["executionIds"],
            "artifactIds": item["artifactIds"],
            "attributions": attributions,
            "fingerprintInputs": item["fingerprintInputs"],
            "logs": {
                "stdout": paths["stdout"].relative_to(ROOT).as_posix(),
                "stderr": paths["stderr"].relative_to(ROOT).as_posix(),
            },
        }
        paths["receipt"].write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        results.append({**receipt, "cacheHit": hit})
        if exit_code != 0:
            print(paths["stderr"].read_text(encoding="utf-8")[-8000:], file=sys.stderr)
            return exit_code, {"schemaVersion": 1, "valid": False, "executions": results}
    return 0, {"schemaVersion": 1, "valid": True, "executions": results}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--execution", action="append", dest="executions")
    parser.add_argument("--attribution", action="append", default=[])
    parser.add_argument("--plan", action="store_true")
    args = parser.parse_args()
    try:
        plan = make_plan(args.executions, args.attribution)
        if args.plan:
            print(json.dumps({"schemaVersion": 1, "executions": plan}, indent=2))
            return 0
        exit_code, report = run_plan(plan)
    except ExecutionError as error:
        print(f"proof execution: {error}", file=sys.stderr)
        return 1
    CACHE_ROOT.mkdir(parents=True, exist_ok=True)
    report_path = CACHE_ROOT / "latest-report.json"
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if exit_code == 0:
        shared = sum(len(item["attributions"]) > 1 for item in report["executions"])
        print(
            f"Real proof execution: OK ({len(report['executions'])} executions, "
            f"{shared} shared, report {report_path.relative_to(ROOT)})"
        )
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
