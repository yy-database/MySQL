// Truth-corpus capture from a real `mysql:8.4` instance and comparison against
// this implementation.
//
// Capture is the only writer of `corpus/truth.json`, and it refuses to
// overwrite a frozen corpus unless `TRUTH_CAPTURE_REFRESH=1` is set. Verify only
// reads the corpus: this implementation's output is never written back.
//
// Modes:
//   node stages/truth.mjs            auto: verify a frozen corpus, or capture to
//                                    a temporary file first when none is frozen
//   node stages/truth.mjs capture    capture to `corpus/truth.json`
//   node stages/truth.mjs verify     compare against `corpus/truth.json`

import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import mysql from 'mysql2/promise';

const require = createRequire(import.meta.url);
const harness = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const corpusPath = join(harness, 'corpus', 'truth.json');
const driverVersion = require('mysql2/package.json').version;

const ROOT_PASSWORD = 'yydb-truth';
const DATABASE = 'yydb';

// `compare: "columns"` is used where the real server may legitimately differ in
// value (version banner, autocommit spelling) but the column contract must hold.
const SCENARIOS = [
    {
        name: 'semantics',
        restart: false,
        steps: [
            { sql: 'SELECT 1', compare: 'columns' },
            { sql: 'SELECT @@version AS version', compare: 'columns' },
            { sql: 'SET autocommit = 0', compare: 'columns' },
            { sql: 'SELECT @@autocommit AS autocommit', compare: 'columns' },
            { sql: 'CREATE TABLE truth_rows (id INT, name VARCHAR(32))', compare: 'columns' },
            { sql: "INSERT INTO truth_rows VALUES (1, 'alpha')", compare: 'full' },
            { sql: 'SELECT id, name FROM truth_rows', compare: 'full' },
            { sql: "UPDATE truth_rows SET name = 'alpha-updated' WHERE id = 1", compare: 'full' },
            { sql: 'SELECT name FROM truth_rows WHERE id = 1', compare: 'full' },
        ],
        after: [],
    },
    {
        name: 'persistence',
        restart: true,
        steps: [
            { sql: 'SET autocommit = 1', compare: 'columns' },
            { sql: 'CREATE TABLE truth_keep (id INT, name VARCHAR(32))', compare: 'columns' },
            { sql: "INSERT INTO truth_keep VALUES (2, 'beta')", compare: 'full' },
        ],
        after: [{ sql: 'SELECT id, name FROM truth_keep', compare: 'full' }],
    },
];

function json(value) {
    return JSON.stringify(value);
}

function docker(args, options = {}) {
    return execFileSync('docker', args, { encoding: 'utf8', ...options });
}

function hasCommand(command) {
    const probe = spawnSync(command, ['--version'], { encoding: 'utf8' });
    return !probe.error;
}

async function freePort() {
    const server = createServer();
    await new Promise((done) => server.listen(0, '127.0.0.1', done));
    const { port } = server.address();
    await new Promise((done) => server.close(done));
    return port;
}

async function sleep(ms) {
    await new Promise((done) => setTimeout(done, ms));
}

async function waitForMysql(port, user, password) {
    const deadline = Date.now() + 60_000;
    let lastError = 'no attempt';
    while (Date.now() < deadline) {
        try {
            const connection = await mysql.createConnection({ host: '127.0.0.1', port, user, password, database: DATABASE });
            await connection.query('SELECT 1');
            await connection.end();
            return;
        } catch (error) {
            lastError = error.message;
            await sleep(1_000);
        }
    }
    throw new Error(`mysql:8.4 did not become ready within 60s: ${lastError}`);
}

async function runQuery(connection, step) {
    const [result, fields] = await connection.query({ sql: step.sql, rowsAsArray: true });
    if (Array.isArray(result)) {
        return { sql: step.sql, compare: step.compare, columns: fields.map((field) => field.name), rows: result };
    }
    return { sql: step.sql, compare: step.compare, columns: null, rows: [], affectedRows: result.affectedRows };
}

async function capture(target) {
    if (!hasCommand('docker')) throw new Error('docker unavailable: the truth corpus needs a real mysql:8.4 container');
    if (existsSync(target) && process.env.TRUTH_CAPTURE_REFRESH !== '1') {
        throw new Error(`refusing to overwrite the frozen corpus at ${target}; set TRUTH_CAPTURE_REFRESH=1 to recapture`);
    }

    const name = `yydb-truth-${process.pid}-${Date.now()}`;
    const port = await freePort();
    docker([
        'run',
        '-d',
        '--name',
        name,
        '-e',
        `MYSQL_ROOT_PASSWORD=${ROOT_PASSWORD}`,
        '-e',
        `MYSQL_DATABASE=${DATABASE}`,
        '-p',
        `127.0.0.1:${port}:3306`,
        'mysql:8.4',
    ]);

    try {
        await waitForMysql(port, 'root', ROOT_PASSWORD);
        let digest = null;
        try {
            digest = docker(['image', 'inspect', 'mysql:8.4', '--format', '{{index .RepoDigests 0}}']).trim() || null;
        } catch {
            digest = null;
        }

        const connection = await mysql.createConnection({ host: '127.0.0.1', port, user: 'root', password: ROOT_PASSWORD, database: DATABASE });
        const [versionRows] = await connection.query('SELECT VERSION() AS version');
        const scenarios = [];
        let open = connection;
        for (const scenario of SCENARIOS) {
            const steps = [];
            for (const step of scenario.steps) steps.push(await runQuery(open, step));
            const after = [];
            if (scenario.restart) {
                await open.end();
                docker(['stop', name]);
                docker(['start', name]);
                await waitForMysql(port, 'root', ROOT_PASSWORD);
                open = await mysql.createConnection({ host: '127.0.0.1', port, user: 'root', password: ROOT_PASSWORD, database: DATABASE });
                for (const step of scenario.after) after.push(await runQuery(open, step));
            }
            scenarios.push({ name: scenario.name, restart: scenario.restart, steps, after });
        }
        await open.end();

        const corpus = {
            provenance: {
                image: 'mysql:8.4',
                imageDigest: digest,
                serverVersion: versionRows[0].version,
                capturedAt: new Date().toISOString(),
                driver: 'mysql2',
                driverVersion,
                captureTool: 'projects/mysql.ts/packages/mysql-harness/stages/truth.mjs',
            },
            scenarios,
        };
        writeFileSync(target, `${JSON.stringify(corpus, null, 4)}\n`);
        console.log(`captured ${target} from mysql:8.4 (${corpus.provenance.serverVersion})`);
    } finally {
        try {
            docker(['rm', '-f', name]);
        } catch {
            // The container is already gone; nothing else to clean.
        }
    }
}

function compareStep(expected, observed, label, counter) {
    counter.total += 1;
    const columnsMatch = json(observed.columns) === json(expected.columns);
    const fullMatch =
        columnsMatch &&
        json(observed.rows) === json(expected.rows) &&
        (observed.affectedRows ?? null) === (expected.affectedRows ?? null);
    const matched = expected.compare === 'full' ? fullMatch : columnsMatch;
    if (matched) {
        counter.passed += 1;
        console.log(`PASS ${label}: ${expected.sql}`);
        return;
    }
    counter.failures.push(label);
    console.log(
        `FAIL ${label}: ${expected.sql} observed=${json({ columns: observed.columns, rows: observed.rows, affectedRows: observed.affectedRows })} expected=${json({ columns: expected.columns, rows: expected.rows, affectedRows: expected.affectedRows })}`,
    );
}

async function verify(source) {
    const corpus = JSON.parse(readFileSync(source, 'utf8'));
    const { startServerProcess, stopServerProcess } = await import('../src/server.mjs');
    const counter = { passed: 0, total: 0, failures: [] };
    const root = mkdtempSync(join(tmpdir(), 'mysql-truth-'));

    for (const scenario of corpus.scenarios) {
        const dataDir = join(root, scenario.name);
        let running = await startServerProcess({ dataDir });
        try {
            let connection = await connectWithRetry(running.address);
            for (const step of scenario.steps) {
                compareStep(step, await runQuery(connection, step), `${scenario.name}`, counter);
            }
            if (scenario.restart) {
                await connection.end();
                connection = null;
                await stopServerProcess(running.child);
                running = await startServerProcess({ dataDir });
                connection = await connectWithRetry(running.address);
                for (const step of scenario.after) {
                    compareStep(step, await runQuery(connection, step), `${scenario.name} after restart`, counter);
                }
            }
            if (scenario.name === 'semantics') probeCli(running.address, counter);
            if (connection) await connection.end();
        } finally {
            await stopServerProcess(running.child);
        }
    }

    console.log(`mysql-harness (truth): ${counter.passed}/${counter.total} assertions, ${corpus.scenarios.length} scenarios, 0 skipped`);
    if (counter.failures.length > 0) {
        for (const failure of counter.failures) console.log(`FAILED ${failure}`);
        throw new Error(`${counter.failures.length} truth assertions failed`);
    }
}

async function connectWithRetry(address) {
    const hostPort = address.split(':');
    const host = hostPort[0];
    const port = Number(hostPort[1]);
    const deadline = Date.now() + 5_000;
    let lastError = 'no attempt';
    while (Date.now() < deadline) {
        try {
            return await mysql.createConnection({ host, port, user: 'yydb', password: '', database: DATABASE });
        } catch (error) {
            lastError = error.message;
            await sleep(200);
        }
    }
    throw new Error(`cannot connect to the composed server at ${address}: ${lastError}`);
}

function probeCli(address, counter) {
    const hostPort = address.split(':');
    counter.total += 1;
    if (!hasCommand('mysql')) {
        counter.failures.push('mysql cli');
        console.log('FAIL mysql cli: the `mysql` client is unavailable');
        return;
    }
    try {
        const output = execFileSync(
            'mysql',
            [
                '--ssl-mode=DISABLED',
                '-h',
                hostPort[0],
                '-P',
                hostPort[1],
                '-u',
                'yydb',
                '--password=',
                '--connect-timeout=5',
                '-e',
                'SELECT 1',
            ],
            { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] },
        );
        if (output.includes('1')) {
            counter.passed += 1;
            console.log('PASS mysql cli: SELECT 1 through the mysql client');
            return;
        }
        throw new Error(`unexpected output: ${output.trim()}`);
    } catch (error) {
        counter.failures.push('mysql cli');
        console.log(`FAIL mysql cli: ${error.message}`);
    }
}

async function main() {
    const mode = process.argv[2] ?? 'auto';
    try {
        if (mode === 'capture') {
            await capture(corpusPath);
            return;
        }
        if (mode === 'verify') {
            await verify(corpusPath);
            return;
        }
        if (existsSync(corpusPath)) {
            await verify(corpusPath);
            return;
        }
        const temporary = join(mkdtempSync(join(tmpdir(), 'mysql-corpus-')), 'truth.json');
        await capture(temporary);
        await verify(temporary);
        console.log('truth corpus is not frozen yet; captured into a temporary file and verified');
    } catch (error) {
        console.error(`FAIL truth stage: ${error.message}`);
        process.exit(1);
    }
}

await main();
