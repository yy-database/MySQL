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
    // WHERE filters, ordering, LIMIT/OFFSET and the row counts every write
    // reports. `affectedRows` is captured as the wire number so a client that
    // negotiates `CLIENT_FOUND_ROWS` (mysql2 does) is pinned to matched rows.
    {
        name: 'where-order',
        restart: false,
        steps: [
            { sql: 'CREATE TABLE truth_wo (id INT NOT NULL PRIMARY KEY, name VARCHAR(32), score INT, team INT)', compare: 'columns' },
            {
                sql: "INSERT INTO truth_wo VALUES (1, 'ann', 10, 1), (2, 'bob', 20, 1), (3, 'cid', 30, 2), (4, 'dan', NULL, 2), (5, 'eve', 20, NULL)",
                compare: 'full',
            },
            { sql: 'SELECT id, name FROM truth_wo WHERE score > 15 ORDER BY score DESC, id', compare: 'full' },
            { sql: 'SELECT id FROM truth_wo WHERE score = 20 ORDER BY id', compare: 'full' },
            { sql: 'SELECT id FROM truth_wo WHERE score IS NULL ORDER BY id', compare: 'full' },
            { sql: 'SELECT id FROM truth_wo WHERE score IS NOT NULL ORDER BY id DESC', compare: 'full' },
            { sql: "SELECT id FROM truth_wo WHERE name LIKE 'a%' ORDER BY id", compare: 'full' },
            { sql: 'SELECT id FROM truth_wo WHERE id IN (1, 3) ORDER BY id', compare: 'full' },
            { sql: 'SELECT id FROM truth_wo WHERE id BETWEEN 2 AND 4 ORDER BY id', compare: 'full' },
            { sql: 'SELECT id FROM truth_wo WHERE score > 10 AND team = 1 ORDER BY id', compare: 'full' },
            { sql: 'SELECT id FROM truth_wo WHERE score > 25 OR team = 1 ORDER BY id', compare: 'full' },
            { sql: 'SELECT id FROM truth_wo WHERE team <> 1 ORDER BY id', compare: 'full' },
            { sql: 'SELECT name FROM truth_wo ORDER BY name DESC LIMIT 2', compare: 'full' },
            { sql: 'SELECT name FROM truth_wo ORDER BY name LIMIT 2 OFFSET 1', compare: 'full' },
            { sql: 'UPDATE truth_wo SET score = 99 WHERE id = 999', compare: 'full' },
            { sql: 'UPDATE truth_wo SET score = 99 WHERE id = 2', compare: 'full' },
            { sql: 'SELECT score FROM truth_wo WHERE id = 2', compare: 'full' },
            { sql: 'UPDATE truth_wo SET score = 0 WHERE team = 2', compare: 'full' },
            { sql: 'DELETE FROM truth_wo WHERE id = 999', compare: 'full' },
            { sql: 'DELETE FROM truth_wo WHERE team = 2', compare: 'full' },
            { sql: 'SELECT COUNT(*) AS n FROM truth_wo', compare: 'full' },
        ],
        after: [],
    },
    // Aggregates, GROUP BY, HAVING and DISTINCT over a table whose nullable
    // column exercises SQL three-valued logic in every aggregate.
    {
        name: 'aggregates',
        restart: false,
        steps: [
            { sql: 'CREATE TABLE truth_agg (id INT NOT NULL PRIMARY KEY, team INT, score INT)', compare: 'columns' },
            { sql: 'INSERT INTO truth_agg VALUES (1, 1, 10), (2, 1, 20), (3, 2, 30), (4, 2, NULL), (5, NULL, 20), (6, 3, NULL)', compare: 'full' },
            { sql: 'SELECT COUNT(*) AS n, COUNT(score) AS ns, COUNT(team) AS nt FROM truth_agg', compare: 'full' },
            { sql: 'SELECT SUM(score) AS s, MIN(score) AS mi, MAX(score) AS ma FROM truth_agg', compare: 'full' },
            { sql: 'SELECT SUM(score) AS s FROM truth_agg WHERE score > 1000', compare: 'full' },
            { sql: 'SELECT team, COUNT(*) AS n, SUM(score) AS s FROM truth_agg GROUP BY team ORDER BY team', compare: 'full' },
            { sql: 'SELECT team, COUNT(*) AS n FROM truth_agg GROUP BY team HAVING COUNT(*) > 1 ORDER BY team', compare: 'full' },
            { sql: 'SELECT DISTINCT score FROM truth_agg ORDER BY score', compare: 'full' },
            { sql: 'SELECT MAX(score) AS ma FROM truth_agg WHERE team = 3', compare: 'full' },
        ],
        after: [],
    },
    // INNER / LEFT / RIGHT joins, including rows whose join key has no match.
    {
        name: 'joins',
        restart: false,
        steps: [
            { sql: 'CREATE TABLE truth_dept (id INT NOT NULL PRIMARY KEY, dname VARCHAR(32))', compare: 'columns' },
            { sql: 'CREATE TABLE truth_emp (id INT NOT NULL PRIMARY KEY, name VARCHAR(32), dept INT, salary INT)', compare: 'columns' },
            { sql: "INSERT INTO truth_dept VALUES (1, 'red'), (2, 'blue'), (3, 'green')", compare: 'full' },
            { sql: "INSERT INTO truth_emp VALUES (1, 'ann', 1, 100), (2, 'bob', 1, 200), (3, 'cid', 2, 300), (4, 'dan', NULL, 400), (5, 'eve', 9, 500)", compare: 'full' },
            { sql: 'SELECT e.id, e.name, d.dname FROM truth_emp e JOIN truth_dept d ON e.dept = d.id ORDER BY e.id', compare: 'full' },
            { sql: 'SELECT e.id, d.dname FROM truth_emp e LEFT JOIN truth_dept d ON e.dept = d.id ORDER BY e.id', compare: 'full' },
            { sql: 'SELECT d.id, e.name FROM truth_emp e RIGHT JOIN truth_dept d ON e.dept = d.id ORDER BY d.id, e.id', compare: 'full' },
            { sql: 'SELECT COUNT(*) AS n FROM truth_emp e JOIN truth_dept d ON e.dept = d.id', compare: 'full' },
            { sql: 'SELECT e.name FROM truth_emp e JOIN truth_dept d ON e.dept = d.id WHERE d.dname = \'red\' ORDER BY e.name', compare: 'full' },
        ],
        after: [],
    },
    // Declared-type boundaries and NULL semantics on every supported column
    // family. Values stay inside each declared range so the corpus pins
    // formatting, roundtrip and NULL handling rather than overflow errors.
    {
        name: 'types',
        restart: false,
        steps: [
            {
                sql: 'CREATE TABLE truth_types (id INT NOT NULL PRIMARY KEY, ti TINYINT, si SMALLINT, bi BIGINT, fl FLOAT, db DOUBLE, vc VARCHAR(8), tx TEXT, bo BOOLEAN, d DATE, dt DATETIME)',
                compare: 'columns',
            },
            {
                sql: "INSERT INTO truth_types VALUES (1, 127, 32767, 9223372036854775807, 1.5, 2.25, 'abc', 'hello', 1, '2024-01-02', '2024-01-02 03:04:05')",
                compare: 'full',
            },
            { sql: 'INSERT INTO truth_types (id, ti, si, bi, fl, db, vc, tx, bo, d, dt) VALUES (2, -128, -32768, -9223372036854775807, -0.5, -2.25, \'\', \'\', 0, \'1970-01-01\', \'1970-01-01 00:00:00\')', compare: 'full' },
            { sql: 'INSERT INTO truth_types (id) VALUES (3)', compare: 'full' },
            { sql: 'SELECT id, ti, si, bi FROM truth_types ORDER BY id', compare: 'full' },
            { sql: 'SELECT id, fl, db FROM truth_types ORDER BY id', compare: 'full' },
            { sql: 'SELECT id, vc, tx, LENGTH(vc) AS vlen FROM truth_types ORDER BY id', compare: 'full' },
            { sql: 'SELECT id, bo FROM truth_types ORDER BY id', compare: 'full' },
            { sql: 'SELECT id, d, dt FROM truth_types ORDER BY id', compare: 'full' },
            { sql: 'SELECT id FROM truth_types WHERE bi > 0', compare: 'full' },
            { sql: "SELECT id FROM truth_types WHERE vc = ''", compare: 'full' },
            { sql: 'SELECT id FROM truth_types WHERE vc IS NULL ORDER BY id', compare: 'full' },
            { sql: 'SELECT id FROM truth_types WHERE bo = TRUE', compare: 'full' },
            { sql: "SELECT id FROM truth_types WHERE d = '2024-01-02'", compare: 'full' },
            { sql: 'SELECT id FROM truth_types WHERE ti BETWEEN -128 AND 127 ORDER BY id', compare: 'full' },
            { sql: 'SELECT COUNT(*) AS n, COUNT(vc) AS nv, COUNT(tx) AS nt FROM truth_types', compare: 'full' },
        ],
        after: [],
    },
    // Error contract: only the MySQL error number and SQLSTATE are compared.
    // Message text is server-specific by design and is deliberately not pinned.
    {
        name: 'errors',
        restart: false,
        steps: [
            { sql: 'CREATE TABLE truth_err (id INT NOT NULL PRIMARY KEY, v INT NOT NULL)', compare: 'columns' },
            { sql: 'SELECT id FROM truth_err_missing', compare: 'full' },
            { sql: 'SELECT missing_col FROM truth_err', compare: 'full' },
            { sql: 'INSERT INTO truth_err VALUES (1, 5)', compare: 'full' },
            { sql: 'INSERT INTO truth_err VALUES (1, 6)', compare: 'full' },
            { sql: 'INSERT INTO truth_err VALUES (2, NULL)', compare: 'full' },
            { sql: 'INSERT INTO truth_err VALUES (3, \'not-a-number\')', compare: 'full' },
            { sql: 'INSERT INTO truth_err (id) VALUES (4)', compare: 'full' },
            { sql: 'SELEC 1', compare: 'full' },
            { sql: 'SELECT COUNT(*) AS n FROM truth_err', compare: 'full' },
        ],
        after: [],
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
            // `timezone: 'Z'` keeps DATE / DATETIME parsing independent of the
            // runner's local zone so the frozen ISO values verify anywhere.
            const connection = await mysql.createConnection({ host: '127.0.0.1', port, user, password, database: DATABASE, timezone: 'Z' });
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
    let result;
    let fields;
    try {
        [result, fields] = await connection.query({ sql: step.sql, rowsAsArray: true });
    } catch (error) {
        // Only the machine-readable contract is pinned. Message text is
        // server-specific and would make the corpus unportable.
        return {
            sql: step.sql,
            compare: step.compare,
            columns: null,
            rows: [],
            error: { errno: error.errno ?? null, sqlState: error.sqlState ?? null },
        };
    }
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

        const connection = await mysql.createConnection({ host: '127.0.0.1', port, user: 'root', password: ROOT_PASSWORD, database: DATABASE, timezone: 'Z' });
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
                open = await mysql.createConnection({ host: '127.0.0.1', port, user: 'root', password: ROOT_PASSWORD, database: DATABASE, timezone: 'Z' });
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
    const errorMatch = json(observed.error ?? null) === json(expected.error ?? null);
    const fullMatch =
        columnsMatch &&
        errorMatch &&
        json(observed.rows) === json(expected.rows) &&
        (observed.affectedRows ?? null) === (expected.affectedRows ?? null);
    const matched = expected.compare === 'full' ? fullMatch : columnsMatch && errorMatch;
    if (matched) {
        counter.passed += 1;
        console.log(`PASS ${label}: ${expected.sql}`);
        return;
    }
    counter.failures.push(label);
    console.log(
        `FAIL ${label}: ${expected.sql} observed=${json({ columns: observed.columns, rows: observed.rows, affectedRows: observed.affectedRows, error: observed.error })} expected=${json({ columns: expected.columns, rows: expected.rows, affectedRows: expected.affectedRows, error: expected.error })}`,
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
            return await mysql.createConnection({ host, port, user: 'yydb', password: '', database: DATABASE, timezone: 'Z' });
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

// 影子模式已在钉入 `database.harness` 后连续 3 轮逐字节一致；冻结语料没有台账与
// 摘要消费者，脚手架随切换一并移除。摘要原语 `harnessDigest` 仍由绑定导出，供
// 未来语料身份门使用。
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
