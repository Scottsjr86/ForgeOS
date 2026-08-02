#!/usr/bin/env python3
"""Run the required jobs declared in ci/master.yaml."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import re
import shlex
import subprocess
import sys
from pathlib import Path
from typing import Any

try:
    import yaml  # type: ignore[import-untyped]
except ModuleNotFoundError:
    sys.stderr.write(
        "[ci] PyYAML is required to read ci/master.yaml.\n"
        "     Install it with: python3 -m pip install PyYAML\n"
    )
    raise SystemExit(2)


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONFIG = REPO_ROOT / "ci" / "master.yaml"
CARGO_TEST_RESULT = re.compile(
    r"^test result: [^.]+\. "
    r"(?P<passed>\d+) passed; "
    r"(?P<failed>\d+) failed; "
    r"(?P<ignored>\d+) ignored; "
    r"(?P<measured>\d+) measured; "
    r"(?P<filtered_out>\d+) filtered out;"
)


class ConfigError(ValueError):
    """Raised when the CI configuration does not match the supported schema."""


@dataclass(frozen=True)
class CommandResult:
    job: str
    index: int
    command: str
    returncode: int
    last_line: str


def _load_config(path: Path) -> dict[str, Any]:
    try:
        config = yaml.safe_load(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise ConfigError(f"cannot read {path}: {error}") from error
    except yaml.YAMLError as error:
        raise ConfigError(f"cannot parse {path}: {error}") from error

    if not isinstance(config, dict):
        raise ConfigError(f"{path} must contain a mapping at the top level")
    if config.get("version") != 1:
        raise ConfigError(f"{path} must declare version: 1")
    return config


def _required_jobs(config: dict[str, Any]) -> list[tuple[str, list[str]]]:
    jobs = config.get("jobs")
    gates = config.get("gates")
    if not isinstance(jobs, dict):
        raise ConfigError("jobs must be a mapping")
    if not isinstance(gates, dict):
        raise ConfigError("gates must be a mapping")

    must_pass = gates.get("must_pass")
    if not isinstance(must_pass, list) or not must_pass:
        raise ConfigError("gates.must_pass must be a non-empty list")

    required: list[tuple[str, list[str]]] = []
    for job_name in must_pass:
        if not isinstance(job_name, str) or not job_name:
            raise ConfigError("gates.must_pass entries must be non-empty strings")
        job = jobs.get(job_name)
        if not isinstance(job, dict):
            raise ConfigError(f"required job '{job_name}' is not defined")
        runs = job.get("runs")
        if not isinstance(runs, list) or not runs:
            raise ConfigError(f"job '{job_name}' runs must be a non-empty list")
        if not all(isinstance(command, str) and command.strip() for command in runs):
            raise ConfigError(f"job '{job_name}' contains an invalid command")
        required.append((job_name, runs))

    return required


def _run_command(job: str, index: int, command: str) -> CommandResult:
    try:
        argv = shlex.split(command)
    except ValueError as error:
        raise ConfigError(f"job '{job}' command {index} is invalid: {error}") from error
    if not argv:
        raise ConfigError(f"job '{job}' command {index} is empty")

    print(f"\n===== CI JOB [{job}] CMD [{index}] =====")
    print(f"$ {command}", flush=True)

    try:
        process = subprocess.Popen(
            argv,
            cwd=REPO_ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        )
    except OSError as error:
        line = f"unable to start command: {error}"
        print(f"[ci] {line}", file=sys.stderr)
        return CommandResult(job, index, command, 127, line)

    last_line = "(no output)"
    cargo_test_totals = {
        "suites": 0,
        "passed": 0,
        "failed": 0,
        "ignored": 0,
        "measured": 0,
        "filtered_out": 0,
    }
    assert process.stdout is not None
    for line in process.stdout:
        sys.stdout.write(line)
        sys.stdout.flush()
        candidate = line.rstrip("\r\n")
        if candidate.strip():
            last_line = candidate

        match = CARGO_TEST_RESULT.match(candidate.strip())
        if match:
            cargo_test_totals["suites"] += 1
            for field in ("passed", "failed", "ignored", "measured", "filtered_out"):
                cargo_test_totals[field] += int(match.group(field))

    returncode = process.wait()
    if argv[:2] == ["cargo", "test"] and cargo_test_totals["suites"]:
        status = "PASS" if returncode == 0 else "FAIL"
        last_line = (
            f"CARGO_TEST_SUMMARY status={status} "
            f"suites={cargo_test_totals['suites']} "
            f"passed={cargo_test_totals['passed']} "
            f"failed={cargo_test_totals['failed']} "
            f"ignored={cargo_test_totals['ignored']} "
            f"measured={cargo_test_totals['measured']} "
            f"filtered_out={cargo_test_totals['filtered_out']}"
        )

    return CommandResult(job, index, command, returncode, last_line)


def _print_last_lines(results: list[CommandResult]) -> None:
    print("\n===== CI COMMAND SUMMARIES =====")
    if not results:
        print("(no commands ran)")
        return

    for result in results:
        status = "PASS" if result.returncode == 0 else f"FAIL({result.returncode})"
        print(f"[{status}] {result.job} command {result.index}: {result.last_line}")


def run_ci(config_path: Path) -> int:
    config = _load_config(config_path)
    jobs = _required_jobs(config)
    gates = config["gates"]
    fail_fast = gates.get("fail_fast", True)
    if not isinstance(fail_fast, bool):
        raise ConfigError("gates.fail_fast must be a boolean")

    results: list[CommandResult] = []
    failed = False
    for job_name, commands in jobs:
        for index, command in enumerate(commands, start=1):
            result = _run_command(job_name, index, command)
            results.append(result)
            if result.returncode != 0:
                failed = True
                print(
                    f"[ci] job '{job_name}' command {index} failed "
                    f"with exit code {result.returncode}",
                    file=sys.stderr,
                )
                if fail_fast:
                    _print_last_lines(results)
                    return result.returncode

    _print_last_lines(results)
    print("CI RESULT: " + ("FAIL" if failed else "PASS"))
    return 1 if failed else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Run ForgeOS CI jobs")
    parser.add_argument(
        "--config",
        type=Path,
        default=DEFAULT_CONFIG,
        help="CI configuration path (default: ci/master.yaml)",
    )
    args = parser.parse_args(argv)

    config_path = args.config
    if not config_path.is_absolute():
        config_path = REPO_ROOT / config_path

    try:
        return run_ci(config_path)
    except ConfigError as error:
        print(f"[ci] configuration error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
