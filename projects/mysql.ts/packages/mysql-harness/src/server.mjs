// Child-process control for the harness. The server always runs in its own
// process so a restart case proves that state survives a process boundary.

import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { loadProduct } from './product.mjs';

const binPath = fileURLToPath(new URL('../bin/mysql-harness.mjs', import.meta.url));

export async function startServer(options) {
    const { startMysqlServer } = await loadProduct();
    return startMysqlServer(options);
}

export async function startServerProcess({ dataDir }) {
    const child = spawn(process.execPath, [binPath, 'serve', '--data-dir', dataDir, '--port', '0'], {
        env: process.env,
        stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stderr = '';
    child.stderr.on('data', (chunk) => {
        stderr += String(chunk);
    });
    const address = await new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
            reject(new Error(`server did not report an address within 10s: ${stderr}`));
        }, 10_000);
        child.stdout.setEncoding('utf8');
        child.stdout.on('data', (chunk) => {
            const match = /^MYSQL_HARNESS_ADDRESS=(.+)$/m.exec(String(chunk));
            if (match) {
                clearTimeout(timer);
                resolve(match[1].trim());
            }
        });
        child.once('exit', (code, signal) => {
            clearTimeout(timer);
            reject(new Error(`server exited during startup (code ${code}, signal ${signal}): ${stderr}`));
        });
    });
    return { child, address };
}

function waitExit(child, timeoutMs) {
    if (child.exitCode !== null || child.signalCode !== null) return Promise.resolve(true);
    return new Promise((resolve) => {
        const timer = setTimeout(() => {
            child.removeListener('exit', onExit);
            resolve(false);
        }, timeoutMs);
        function onExit() {
            clearTimeout(timer);
            resolve(true);
        }
        child.once('exit', onExit);
    });
}

export async function stopServerProcess(child) {
    if (!child || child.exitCode !== null || child.signalCode !== null) return;
    child.kill('SIGTERM');
    if (await waitExit(child, 5_000)) return;
    child.kill('SIGKILL');
    if (!(await waitExit(child, 5_000))) {
        throw new Error('server process did not exit after SIGKILL');
    }
}
