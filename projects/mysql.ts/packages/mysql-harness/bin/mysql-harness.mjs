#!/usr/bin/env node
// Starts the MySQL listener and prints its bound address, then waits for a
// signal. The harness drives this as a child process so "restart" means a real
// process stop, not the same object being stopped and started again.

import { parseArgs } from 'node:util';

import { startServer } from '../src/server.mjs';

const { values } = parseArgs({
    allowPositionals: true,
    options: {
        'data-dir': { type: 'string' },
        port: { type: 'string', default: '0' },
    },
});

if (!values['data-dir']) {
    console.error('mysql-harness serve requires --data-dir <dir>');
    process.exit(2);
}

const server = await startServer({
    dataDir: values['data-dir'],
    port: Number(values.port ?? 0),
});

process.stdout.write(`MYSQL_HARNESS_ADDRESS=${server.address}\n`);

// The listener runs on a Rust worker thread, which does not hold a Node
// handle open. Without a live timer this process would exit right after
// printing the address and every client would see ECONNRESET.
const keepAlive = setInterval(() => {}, 60_000);

let closing = false;
function shutdown() {
    if (closing) return;
    closing = true;
    clearInterval(keepAlive);
    server.stop();
    process.exit(0);
}

process.on('SIGINT', shutdown);
process.on('SIGTERM', shutdown);
