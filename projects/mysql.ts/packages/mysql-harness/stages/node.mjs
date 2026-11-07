// Runs every declarative case against a child-process server.
//
// The restart step stops the child, waits for it to exit and starts a fresh
// process on the same data directory. Nothing here starts the server in-process,
// so "persistence" cannot be satisfied by an in-memory object.

import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import mysql from 'mysql2/promise';

import { startServerProcess, stopServerProcess } from '../src/server.mjs';

const casesDir = fileURLToPath(new URL('../cases/', import.meta.url));
const caseFiles = readdirSync(casesDir)
    .filter((name) => name.endsWith('.json'))
    .sort();

const useTemporaryRoot = !process.env.MYSQL_HARNESS_DATA_DIR;
const root = process.env.MYSQL_HARNESS_DATA_DIR ?? mkdtempSync(join(tmpdir(), 'mysql-harness-'));

let passed = 0;
let total = 0;
const failures = [];

function check(condition, message) {
    total += 1;
    if (condition) {
        passed += 1;
        console.log(`PASS ${message}`);
        return;
    }
    failures.push(message);
    console.log(`FAIL ${message}`);
}

function splitAddress(address) {
    const index = address.lastIndexOf(':');
    if (index < 0) throw new Error(`address ${address} has no port`);
    return [address.slice(0, index), Number(address.slice(index + 1))];
}

async function connect(address) {
    const [host, port] = splitAddress(address);
    return mysql.createConnection({ host, port, user: 'yydb', password: '', charset: 'utf8mb4' });
}

function rowsOf(result, fields) {
    if (!Array.isArray(result) || !Array.isArray(fields)) return null;
    return result.map((row) => fields.map((field) => row[field.name]));
}

async function runStep(connection, step, name) {
    const [result, fields] = await connection.query(step.sql);

    if (step.ok) {
        check(result !== undefined && result !== null, `${name}: ${step.sql} returned an OK payload`);
    }
    if (step.affectedRows !== undefined) {
        check(
            result?.affectedRows === step.affectedRows,
            `${name}: ${step.sql} affectedRows=${result?.affectedRows}, expected ${step.affectedRows}`,
        );
    }
    if (step.columns) {
        const names = (fields ?? []).map((field) => field.name);
        check(
            JSON.stringify(names) === JSON.stringify(step.columns),
            `${name}: ${step.sql} columns=${JSON.stringify(names)}, expected ${JSON.stringify(step.columns)}`,
        );
    }
    if (step.rows) {
        const rows = rowsOf(result, fields) ?? [];
        check(
            JSON.stringify(rows) === JSON.stringify(step.rows),
            `${name}: ${step.sql} rows=${JSON.stringify(rows)}, expected ${JSON.stringify(step.rows)}`,
        );
    }
    if (step.rowCount !== undefined) {
        check(
            Array.isArray(result) && result.length === step.rowCount,
            `${name}: ${step.sql} rowCount=${Array.isArray(result) ? result.length : 'non-result'}, expected ${step.rowCount}`,
        );
    }

    const firstCell = Array.isArray(result) && Array.isArray(fields) && result.length > 0 ? result[0][fields[0].name] : undefined;
    if (step.valueType !== undefined) {
        check(
            typeof firstCell === step.valueType,
            `${name}: ${step.sql} first value type=${typeof firstCell}, expected ${step.valueType}`,
        );
    }
    if (step.valueNonEmpty) {
        check(
            typeof firstCell === 'string' && firstCell.length > 0,
            `${name}: ${step.sql} first value is not empty`,
        );
    }
    if (step.valueIn) {
        const allowed = step.valueIn.some((value) => JSON.stringify(value) === JSON.stringify(firstCell));
        check(allowed, `${name}: ${step.sql} first value=${JSON.stringify(firstCell)}, expected one of ${JSON.stringify(step.valueIn)}`);
    }
}

for (const file of caseFiles) {
    const spec = JSON.parse(readFileSync(join(casesDir, file), 'utf8'));
    const dataDir = join(root, spec.name);
    let child = null;
    let connection = null;
    let address = null;

    try {
        ({ child, address } = await startServerProcess({ dataDir }));
        connection = await connect(address);

        for (const step of spec.steps) {
            if (step.restart) {
                await connection.end();
                connection = null;
                await stopServerProcess(child);
                ({ child, address } = await startServerProcess({ dataDir }));
                connection = await connect(address);
                check(true, `${spec.name}: real process restart on the same data directory`);
                continue;
            }
            if (step.ping) {
                await connection.ping();
                check(true, `${spec.name}: COM_PING accepted`);
                continue;
            }
            await runStep(connection, step, spec.name);
        }
    } catch (error) {
        failures.push(`${spec.name}: ${error?.message ?? error}`);
        console.error(`FAIL ${spec.name}: ${error?.message ?? error}`);
    } finally {
        if (connection) await connection.end().catch(() => {});
        await stopServerProcess(child).catch((error) => {
            failures.push(`${spec.name}: ${error.message}`);
        });
    }
}

const diskDir = join(root, 'ddl-dml-select');
check(existsSync(join(diskDir, 'catalog.yyds')), 'catalog.yyds is written');
check(existsSync(join(diskDir, 'sql-schema.v0.json')), 'sql-schema.v0.json is written');
const shard = existsSync(diskDir) ? readdirSync(diskDir).find((name) => name.endsWith('.yykv')) : undefined;
check(Boolean(shard), 'local-0.yykv is written');
if (shard) {
    check(statSync(join(diskDir, shard)).size > 0, 'local-0.yykv holds row bytes');
}

if (useTemporaryRoot) {
    try {
        rmSync(root, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
    } catch (error) {
        console.error(`cleanup retried but failed: ${error.message}`);
        failures.push(`cleanup: ${error.message}`);
    }
}

console.log(`mysql-harness: ${passed}/${total} assertions, ${caseFiles.length} cases, 0 skipped`);
if (failures.length > 0) {
    for (const failure of failures) console.error(`FAILED ${failure}`);
    process.exit(1);
}
