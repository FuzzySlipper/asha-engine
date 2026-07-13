#!/usr/bin/env python3
"""Run equivalent proof commands once and retain every consuming attribution."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import subprocess
import sys
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


def toolchain_digest(command: list[str]) -> dict[str, str]:
    executable = command[0]
    if executable == "cargo":
        probes = [["cargo", "--version"], ["rustc", "--version", "--verbose"]]
    elif executable == "pnpm":
        probes = [["pnpm", "--version"], ["node", "--version"]]
    else:
        probes = [
            ["bash", "--version"],
            ["cargo", "--version"],
            ["rustc", "--version", "--verbose"],
            ["pnpm", "--version"],
            ["node", "--version"],
        ]
    result: dict[str, str] = {}
    for probe in probes:
        completed = subprocess.run(
            probe, cwd=ROOT, check=False, text=True, capture_output=True
        )
        if completed.returncode != 0:
            raise ExecutionError(f"toolchain probe failed: {' '.join(probe)}")
        result[" ".join(probe)] = completed.stdout.strip()
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
    for identity in selected_ids:
        definition = definitions[identity]
        command = definition["command"]
        toolchain = toolchains.setdefault(tuple(command[:1]), toolchain_digest(command))
        fingerprint, inputs = execution_fingerprint(
            definition,
            settings,
            catalog,
            dict(os.environ if environment is None else environment),
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
