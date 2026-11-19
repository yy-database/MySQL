// Author-facing acceptance for the `MySQL` and `MySQLClient` constructors.
//
// The lifecycle part is real: `open()` must bring the `.yyds` catalog, the
// exclusive lease, the `.yykv` shard and the persisted schema up through the
// native handles, `listen()` must accept a real `mysql2` session that runs DDL,
// DML and SELECT through the upstream gateway, and `MySQLClient.connect()` must
// complete a real handshake against that gateway with text and prepared
// statement round trips through the native client handle. The remaining gaps
// assert the honest failure shape: wherever the native binding exports no
// capability yet (in-process execution, session borrowing, transactions,
// per-request controls), the surface must fail through the error envelope with
// `native_capability_missing` — never a TypeScript fallback.

import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import mysql from 'mysql2/promise';

import { loadProduct } from '../src/product.mjs';

const { MySQL, MySQLClient, MySQLDatabaseError } = await loadProduct();

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

function checkEnvelope(error, { code, phase, execution, retry }, message) {
    const ok =
        error instanceof MySQLDatabaseError &&
        error.product === 'mysql' &&
        error.code === code &&
        error.phase === phase &&
        error.execution === execution &&
        error.retry === retry;
    check(
        ok,
        `${message}: expected MySQLDatabaseError code=${code} phase=${phase} execution=${execution} retry=${retry}, got ${error?.name}: ${error?.code}/${error?.phase}/${error?.execution}/${error?.retry} (${error?.message})`,
    );
}

async function rejection(action, expectation, message) {
    try {
        await action();
        check(false, `${message}: expected a rejection but the call succeeded`);
        return undefined;
    } catch (error) {
        checkEnvelope(error, expectation, message);
        return error;
    }
}

function thrower(action, expectation, message) {
    try {
        action();
        check(false, `${message}: expected a throw but the call returned`);
    } catch (error) {
        checkEnvelope(error, expectation, message);
    }
}

const GAP = { code: 'native_capability_missing', execution: 'not-sent', retry: 'never' };
const CONFIG = { code: '', phase: 'configuration', execution: 'not-sent', retry: 'never' };
const configuration = (code) => ({ ...CONFIG, code });

// --- constructor configuration phase ---------------------------------------

thrower(
    () => new MySQL(),
    configuration('storage_directory_required'),
    'MySQL without storage fails in configuration',
);
thrower(
    () => new MySQL({ storage: { directory: '  ' } }),
    configuration('storage_directory_required'),
    'MySQL with a blank storage directory fails in configuration',
);
thrower(
    () => new MySQL({ storage: { directory: 'x' }, database: '' }),
    configuration('invalid_database'),
    'MySQL with an empty database fails in configuration',
);
thrower(
    () => new MySQLClient({ url: 'mysql://127.0.0.1', endpoint: { host: '127.0.0.1', port: 3306 } }),
    configuration('address_conflict'),
    'MySQLClient rejects url together with endpoint',
);
thrower(
    () => new MySQLClient({ url: 'mysql://127.0.0.1/app', database: 'other' }),
    configuration('address_conflict'),
    'MySQLClient rejects a url database together with a structured database',
);
thrower(() => new MySQLClient({ url: 'postgres://127.0.0.1' }), configuration('invalid_url'), 'MySQLClient rejects a foreign scheme');
thrower(
    () => new MySQLClient({ url: 'mysql://127.0.0.1?ssl=true' }),
    configuration('invalid_url'),
    'MySQLClient rejects unsupported url query parameters',
);
thrower(
    () => new MySQLClient({ endpoint: { host: 'db.internal', port: 0 } }),
    configuration('invalid_endpoint'),
    'MySQLClient rejects an out-of-range endpoint port',
);
thrower(
    () => new MySQLClient({ endpoint: { host: '127.0.0.1', port: 3306 }, charset: 'latin1' }),
    { ...GAP, phase: 'configuration' },
    'MySQLClient refuses charset negotiation instead of silently ignoring it',
);

const urlClient = new MySQLClient({ url: 'mysql://alice:s%40cret@127.0.0.1:3307/app' });
check(urlClient.state === 'new', 'MySQLClient with a valid url constructs in state new');
check(urlClient.endpoint.host === '127.0.0.1' && urlClient.endpoint.port === 3307, 'url host and port are resolved');
check(urlClient.database === 'app', 'url database is resolved');
const defaultClient = new MySQLClient({});
check(defaultClient.endpoint.host === '127.0.0.1' && defaultClient.endpoint.port === 3306, 'MySQLClient defaults to loopback:3306');

// --- local engine lifecycle over the real native handles ---------------------

const useTemporaryRoot = !process.env.MYSQL_HARNESS_DATA_DIR;
const root = process.env.MYSQL_HARNESS_DATA_DIR ?? mkdtempSync(join(tmpdir(), 'mysql-api-'));
const dataDir = join(root, 'authoring');
const mysqlInstance = new MySQL({ storage: { directory: dataDir } });

try {
    await Promise.all([mysqlInstance.open(), mysqlInstance.open()]);
    check(mysqlInstance.state === 'open', 'concurrent open() calls join one operation and end in state open');
    check(existsSync(join(dataDir, 'catalog.yyds')), 'open() publishes the .yyds catalog');
    check(existsSync(join(dataDir, 'node.lock')), 'open() acquires the node lease');
    check(
        readdirSync(dataDir).some((name) => name.endsWith('.yykv')),
        'open() opens the .yykv shard',
    );

    const contender = new MySQL({ storage: { directory: dataDir } });
    await rejection(
        () => contender.open(),
        { code: 'native_open_failed', phase: 'open', execution: 'not-sent', retry: 'application-decision' },
        'a second MySQL over the same directory fails on the exclusive lease',
    );

    const listener = await mysqlInstance.listen();
    check(listener.endpoint.port > 0, 'listen() binds a real port');
    check(listener.address.startsWith('127.0.0.1:'), 'listen() reports a loopback address');

    const connection = await mysql.createConnection({ host: listener.endpoint.host, port: listener.endpoint.port, user: 'yydb', password: '' });
    await connection.query('CREATE TABLE authoring (id BIGINT PRIMARY KEY, name VARCHAR(64))');
    await connection.query("INSERT INTO authoring VALUES (7, 'seven')");
    const [rows] = await connection.query('SELECT id, name FROM authoring WHERE id = 7');
    check(rows.length === 1 && String(rows[0].id) === '7' && rows[0].name === 'seven', 'a real mysql2 session runs DDL, DML and SELECT over the listener');
    check(existsSync(join(dataDir, 'sql-schema.v0.json')), 'the schema registry persists the applied DDL');
    await connection.end();

    const second = await mysqlInstance.listen({ endpoint: { host: '127.0.0.1', port: 0 } });
    check(second.endpoint.port !== listener.endpoint.port, 'a second listen() returns an independent listener');
    const probe = await mysql.createConnection({
        host: second.endpoint.host,
        port: second.endpoint.port,
        user: 'yydb',
        password: '',
    });
    const [probeRows] = await probe.query('SELECT id, name FROM authoring WHERE id = 7');
    check(probeRows.length === 1, 'the second listener serves the same engine');
    await probe.end();

    await listener.close();
    await listener.close();
    check(mysqlInstance.state === 'open', 'closing a listener twice succeeds and leaves the engine open');
    // A closed listener must refuse new sessions while the engine stays up.
    let refused = false;
    try {
        const dead = await mysql.createConnection({ host: '127.0.0.1', port: listener.endpoint.port, user: 'yydb', password: '' });
        await dead.end();
    } catch (error) {
        refused = error?.code === 'ECONNREFUSED';
    }
    check(refused, 'a closed listener stops accepting while the engine stays open');
    const survivor = await mysql.createConnection({ host: '127.0.0.1', port: second.endpoint.port, user: 'yydb', password: '' });
    await survivor.query('SELECT 1');
    await survivor.end();
    check(true, 'the surviving listener still serves the open engine');

    // --- remote client over the real gateway ------------------------------------
    const client = new MySQLClient({ endpoint: second.endpoint, auth: { username: 'yydb', password: '' } });
    await rejection(
        () => client.query('select 1'),
        { code: 'client_not_connected', phase: 'execute', execution: 'not-sent', retry: 'never' },
        'query() before a successful connect fails as not connected',
    );
    await client.connect();
    check(client.state === 'ready', 'connect() completes the real handshake against the gateway');
    const created = await client.execute('CREATE TABLE client_smoke (id BIGINT PRIMARY KEY, name VARCHAR(64))');
    check(
        created.affectedRows === 0n && created.lastInsertId === null && created.warnings.length === 0,
        'DDL reports the OK packet shape',
    );
    const inserted = await client.execute("INSERT INTO client_smoke VALUES (7, 'seven')");
    check(inserted.affectedRows === 1n, 'execute() reports affected rows from the text protocol');
    const selected = await client.query('SELECT id, name FROM client_smoke WHERE id = 7');
    check(
        selected.columns.length === 2 && selected.columns[0].name === 'id' && selected.columns[0].typeCode === 8 && !selected.columns[0].binary,
        'query() exposes the declared column metadata',
    );
    check(
        selected.rows.length === 1 && selected.rows[0].id === 7n && selected.rows[0].name === 'seven',
        'query() restores typed values through the native binding',
    );
    const arrayed = await client.query('SELECT name FROM client_smoke WHERE id = 7', undefined, { arrayRows: true });
    check(Array.isArray(arrayed.rows[0]) && arrayed.rows[0][0] === 'seven', 'arrayRows returns positional rows');
    const prepared = await client.query('SELECT id, name FROM client_smoke WHERE name = ?', ['seven']);
    check(prepared.rows.length === 1 && prepared.rows[0].id === 7n, 'query() with parameters runs the prepared binary protocol');
    const preparedWrite = await client.execute('INSERT INTO client_smoke VALUES (?, ?)', [8, 'eight']);
    check(preparedWrite.affectedRows === 1n, 'execute() binds parameters through the prepared protocol');
    await client.execute('INSERT INTO client_smoke VALUES (?, ?)', [9, null]);
    const nulls = await client.query('SELECT id, name FROM client_smoke WHERE id = 9');
    check(
        nulls.rows.length === 1 && nulls.rows[0].name === null && nulls.rows[0].id === 9n,
        'NULL round-trips through the prepared path and restores as null',
    );
    await rejection(
        () => client.withSession(async () => {}),
        { ...GAP, phase: 'execute' },
        'withSession() stays a capability gap until the native handle exports session borrowing',
    );
    await rejection(
        () => client.transaction(async () => {}),
        { ...GAP, phase: 'execute' },
        'transaction() stays a capability gap until the native handle exports begin/commit/rollback',
    );
    await rejection(
        () => client.query('select 1', undefined, { timeoutMs: 5 }),
        { ...GAP, phase: 'configuration' },
        'a per-request timeout is refused rather than silently ignored',
    );
    await rejection(
        () => client.execute('select 1', undefined, { signal: new AbortController().signal }),
        { ...GAP, phase: 'configuration' },
        'a per-request abort signal is refused rather than silently ignored',
    );
    await client.close();
    check(client.state === 'closed', 'close() drops the native session');
    await client.close();
    check(true, 'repeating client close() succeeds');
    await rejection(
        () => client.connect(),
        { code: 'invalid_state', phase: 'connect', execution: 'not-sent', retry: 'never' },
        'connect() cannot revive a closed client',
    );
    const deadPort = new MySQLClient({ endpoint: { host: '127.0.0.1', port: 1 } });
    await rejection(
        () => deadPort.connect(),
        { code: 'native_connect_failed', phase: 'connect', execution: 'not-sent', retry: 'application-decision' },
        'connect() against a closed port fails through the envelope',
    );
    check(deadPort.state === 'failed', 'a failed handshake leaves the client in state failed');

    await rejection(
        () => mysqlInstance.listen({ endpoint: { host: '0.0.0.0', port: 0 } }),
        configuration('endpoint_host_unsupported'),
        'listen() rejects non-loopback hosts the exported primitive cannot bind',
    );
    await rejection(
        () => mysqlInstance.listen({ authentication: { username: 'app', password: 'secret' } }),
        { ...GAP, phase: 'configuration' },
        'listen() rejects authentication until the primitive exposes it',
    );

    await rejection(() => mysqlInstance.query('select 1'), { ...GAP, phase: 'execute' }, 'query() fails through the capability gap, not a TS engine');
    await rejection(() => mysqlInstance.execute('insert into authoring values (8, 8)'), { ...GAP, phase: 'execute' }, 'execute() fails through the capability gap');
    let sessionCalls = 0;
    await rejection(
        () => mysqlInstance.withSession(async () => { sessionCalls += 1; }),
        { ...GAP, phase: 'execute' },
        'withSession() fails through the capability gap',
    );
    check(sessionCalls === 0, 'withSession() never invokes the callback without a native session handle');
    let transactionCalls = 0;
    await rejection(
        () => mysqlInstance.transaction(async () => { transactionCalls += 1; }),
        { ...GAP, phase: 'execute' },
        'transaction() fails through the capability gap',
    );
    check(transactionCalls === 0, 'transaction() never invokes the callback without native begin/commit/rollback');
    await rejection(
        () => mysqlInstance.query(''),
        configuration('invalid_sql'),
        'query() validates the sql shape before reporting the gap',
    );
    await rejection(
        () => mysqlInstance.query('select ?', [{ nested: true }]),
        configuration('invalid_params'),
        'query() refuses parameter objects: parameters bind through the prepared protocol only',
    );
    await rejection(
        () => mysqlInstance.transaction(async () => {}, { isolation: 'snapshot' }),
        configuration('invalid_isolation'),
        'transaction() rejects an isolation level MySQL does not define',
    );
} finally {
    await mysqlInstance.close().catch((error) => {
        failures.push(`close: ${error.message}`);
    });
}

check(mysqlInstance.state === 'closed', 'close() ends in state closed');
await mysqlInstance.close();
check(true, 'repeating close() succeeds');
await rejection(
    () => mysqlInstance.open(),
    { code: 'invalid_state', phase: 'open', execution: 'not-sent', retry: 'never' },
    'a closed instance never reopens',
);
await rejection(
    () => mysqlInstance.query('select 1'),
    { code: 'invalid_state', phase: 'execute', execution: 'not-sent', retry: 'never' },
    'business calls on a closed instance fail explicitly',
);

// --- remote client lifecycle without a server ---------------------------------

const idleClient = new MySQLClient({ endpoint: { host: '127.0.0.1', port: 3306 } });
await rejection(
    () => idleClient.query('select 1'),
    { code: 'client_not_connected', phase: 'execute', execution: 'not-sent', retry: 'never' },
    'query() before any connect fails as not connected',
);
await rejection(
    () => idleClient.transaction(async () => {}),
    { code: 'client_not_connected', phase: 'execute', execution: 'not-sent', retry: 'never' },
    'transaction() before any connect fails as not connected',
);
await idleClient.close();
check(idleClient.state === 'closed', 'close() on a never-connected client succeeds');
await rejection(
    () => idleClient.connect(),
    { code: 'invalid_state', phase: 'connect', execution: 'not-sent', retry: 'never' },
    'connect() cannot revive a closed client',
);

check(typeof MySQL.prototype[Symbol.asyncDispose] === 'function', 'MySQL provides Symbol.asyncDispose');
check(typeof MySQLClient.prototype[Symbol.asyncDispose] === 'function', 'MySQLClient provides Symbol.asyncDispose');

if (useTemporaryRoot) {
    try {
        rmSync(root, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
    } catch (error) {
        console.error(`cleanup retried but failed: ${error.message}`);
        failures.push(`cleanup: ${error.message}`);
    }
}

console.log(`mysql-api: ${passed}/${total} assertions, 0 skipped`);
if (failures.length > 0) {
    for (const failure of failures) console.error(`FAILED ${failure}`);
    process.exit(1);
}
