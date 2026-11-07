"""Runs every declarative harness case against the installed Python product.

The orchestrator starts a child process for each case and connects with the
real `pymysql` driver. `mysql-connector-python` is not used: it also ships a
top-level `mysql` package, so it cannot be installed next to this product and a
fallback would only produce a half-broken import. If pymysql cannot connect,
the run reports the driver error and fails; it never compares this
implementation against itself.

Usage:
    python run.py --cases <dir> --root <dir>   orchestrator
    python run.py serve --data-dir <dir> --port 0
"""

from __future__ import annotations

import argparse
import json
import os
import queue
import signal
import subprocess
import sys
import threading
import time
from decimal import Decimal
from pathlib import Path

DRIVER_UNAVAILABLE = 3
ADDRESS_PREFIX = "MYSQL_PY_ADDRESS="


class DriverUnavailable(RuntimeError):
    """The real driver could not connect."""


def serve(data_dir: str, port: int) -> int:
    from mysql import start_server

    server = start_server(data_dir, port=port)
    print(f"{ADDRESS_PREFIX}{server.address}", flush=True)
    stopping = threading.Event()

    def stop(_signum: int, _frame: object) -> None:
        stopping.set()

    signal.signal(signal.SIGINT, stop)
    signal.signal(signal.SIGTERM, stop)
    while not stopping.is_set():
        time.sleep(0.1)
    server.stop()
    return 0


class PyMySQLDriver:
    name = "pymysql"

    def __init__(self, connection: object) -> None:
        self.connection = connection

    def query(self, sql: str) -> tuple[list[list[object]], list[str] | None, int]:
        cursor = self.connection.cursor()
        try:
            cursor.execute(sql)
            if cursor.description:
                columns = [column[0] for column in cursor.description]
                rows = [list(row) for row in cursor.fetchall()]
                return rows, columns, len(rows)
            return [], None, int(cursor.rowcount)
        finally:
            cursor.close()

    def ping(self) -> None:
        self.connection.ping(reconnect=False)

    def close(self) -> None:
        self.connection.close()


def connect(address: str) -> object:
    """Connects with the real `pymysql` driver."""
    host, _, port = address.rpartition(":")
    try:
        import pymysql

        return PyMySQLDriver(
            pymysql.connect(
                host=host,
                port=int(port),
                user="yydb",
                password="",
                charset="utf8mb4",
                autocommit=None,
            )
        )
    except Exception as error:  # noqa: BLE001 - the driver error is the report
        raise DriverUnavailable(f"pymysql: {error}") from error


def read_address(process: subprocess.Popen[str], timeout: float = 10.0) -> str:
    lines: queue.Queue[str | None] = queue.Queue()

    def read() -> None:
        assert process.stdout is not None
        lines.put(process.stdout.readline())

    threading.Thread(target=read, daemon=True).start()
    try:
        line = lines.get(timeout=timeout)
    except queue.Empty:
        line = None
    if line is not None and line.startswith(ADDRESS_PREFIX):
        return line[len(ADDRESS_PREFIX) :].strip()
    stderr = ""
    if process.poll() is not None and process.stderr is not None:
        stderr = process.stderr.read().strip()
    raise RuntimeError(f"server did not report an address within {timeout}s: {stderr}")


def start_server(data_dir: str) -> tuple[subprocess.Popen[str], str]:
    process = subprocess.Popen(
        [sys.executable, __file__, "serve", "--data-dir", data_dir, "--port", "0"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        return process, read_address(process)
    except Exception:
        stop_server(process)
        raise


def stop_server(process: subprocess.Popen[str] | None) -> None:
    if process is None or process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=5)
        return
    except subprocess.TimeoutExpired:
        pass
    process.kill()
    process.wait(timeout=5)


def plain(value: object) -> object:
    if isinstance(value, bytes):
        return value.decode("utf-8", "replace")
    if isinstance(value, Decimal):
        return int(value) if value == value.to_integral_value() else float(value)
    if isinstance(value, (list, tuple)):
        return [plain(item) for item in value]
    if hasattr(value, "isoformat"):
        return value.isoformat()
    return value


def type_name(value: object) -> str:
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, (int, float, Decimal)):
        return "number"
    if isinstance(value, str):
        return "string"
    if value is None:
        return "null"
    return type(value).__name__


class Counter:
    def __init__(self) -> None:
        self.passed = 0
        self.total = 0
        self.failures: list[str] = []

    def check(self, condition: bool, message: str) -> None:
        self.total += 1
        if condition:
            self.passed += 1
            print(f"PASS {message}")
            return
        self.failures.append(message)
        print(f"FAIL {message}")


def run_step(driver: object, step: dict[str, object], name: str, counter: Counter) -> None:
    rows, columns, affected = driver.query(step["sql"])
    rows = [plain(row) for row in rows]

    if step.get("ok"):
        counter.check(True, f"{name}: {step['sql']} returned an OK payload")
    if "affectedRows" in step:
        counter.check(
            affected == step["affectedRows"],
            f"{name}: {step['sql']} affectedRows={affected}, expected {step['affectedRows']}",
        )
    if "columns" in step:
        counter.check(
            json.dumps(columns) == json.dumps(step["columns"]),
            f"{name}: {step['sql']} columns={json.dumps(columns)}, expected {json.dumps(step['columns'])}",
        )
    if "rows" in step:
        counter.check(
            json.dumps(rows) == json.dumps(step["rows"]),
            f"{name}: {step['sql']} rows={json.dumps(rows)}, expected {json.dumps(step['rows'])}",
        )
    if "rowCount" in step:
        counter.check(
            len(rows) == step["rowCount"],
            f"{name}: {step['sql']} rowCount={len(rows)}, expected {step['rowCount']}",
        )

    first = rows[0][0] if rows and rows[0] else None
    if "valueType" in step:
        counter.check(
            type_name(first) == step["valueType"],
            f"{name}: {step['sql']} first value type={type_name(first)}, expected {step['valueType']}",
        )
    if step.get("valueNonEmpty"):
        counter.check(
            isinstance(first, str) and len(first) > 0,
            f"{name}: {step['sql']} first value is not empty",
        )
    if "valueIn" in step:
        allowed = any(json.dumps(value) == json.dumps(first) for value in step["valueIn"])
        counter.check(
            allowed,
            f"{name}: {step['sql']} first value={json.dumps(first)}, expected one of {json.dumps(step['valueIn'])}",
        )


def run_case(case_path: Path, root: Path, counter: Counter) -> None:
    spec = json.loads(case_path.read_text(encoding="utf-8"))
    name = spec["name"]
    data_dir = root / name
    process = None
    driver = None
    try:
        process, address = start_server(str(data_dir))
        driver = connect(address)
        for step in spec["steps"]:
            if step.get("restart"):
                driver.close()
                driver = None
                stop_server(process)
                process = None
                process, address = start_server(str(data_dir))
                driver = connect(address)
                counter.check(True, f"{name}: real process restart on the same data directory")
                continue
            if step.get("ping"):
                driver.ping()
                counter.check(True, f"{name}: COM_PING accepted")
                continue
            run_step(driver, step, name, counter)
    except DriverUnavailable:
        raise
    except Exception as error:  # noqa: BLE001 - every case failure is recorded
        counter.failures.append(f"{name}: {error}")
        print(f"FAIL {name}: {error}")
    finally:
        if driver is not None:
            try:
                driver.close()
            except Exception:  # noqa: BLE001 - closing a dead socket is not a failure
                pass
        stop_server(process)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", nargs="?", default="run", choices=["run", "serve"])
    parser.add_argument("--cases", type=Path)
    parser.add_argument("--root", type=Path)
    parser.add_argument("--data-dir", type=Path)
    parser.add_argument("--port", type=int, default=0)
    args = parser.parse_args()

    if args.mode == "serve":
        return serve(str(args.data_dir), args.port)

    spec_root = args.root
    spec_root.mkdir(parents=True, exist_ok=True)
    counter = Counter()
    case_files = sorted(args.cases.glob("*.json"))
    try:
        for case_file in case_files:
            run_case(case_file, spec_root, counter)
    except DriverUnavailable as error:
        print(f"driver unavailable: {error}", file=sys.stderr)
        return DRIVER_UNAVAILABLE

    disk_dir = spec_root / "ddl-dml-select"
    counter.check((disk_dir / "catalog.yyds").exists(), "catalog.yyds is written")
    counter.check((disk_dir / "sql-schema.v0.json").exists(), "sql-schema.v0.json is written")
    shards = sorted(disk_dir.glob("*.yykv")) if disk_dir.exists() else []
    counter.check(bool(shards), "local-0.yykv is written")
    if shards:
        counter.check(shards[0].stat().st_size > 0, "local-0.yykv holds row bytes")

    print(f"mysql-harness (python): {counter.passed}/{counter.total} assertions, {len(case_files)} cases, 0 skipped")
    if counter.failures:
        for failure in counter.failures:
            print(f"FAILED {failure}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
