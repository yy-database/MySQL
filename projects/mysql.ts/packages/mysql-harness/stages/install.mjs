// Install-state acceptance.
//
// Packs `@yydb/mysql`, the matching platform package and this harness into a
// system temporary directory, installs them with npm and runs `stages/node.mjs`
// with plain `node` against the installed dist. Nothing resolves from the
// workspace source tree at run time.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const harnessDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repo = resolve(harnessDir, '..', '..', '..', '..');
const platform = `${process.platform}-${process.arch}`;
const productDir = join(repo, 'projects', 'mysql.ts', 'packages', 'mysql');
const platformDir = join(repo, 'projects', 'mysql.ts', 'platforms', `mysql-${platform}`);

const supported = new Set(['win32-x64', 'linux-x64', 'darwin-x64', 'darwin-arm64']);
if (!supported.has(platform)) {
    throw new Error(`install-state acceptance does not support platform ${platform}`);
}

const productEntry = join(productDir, 'dist', 'index.js');
if (!existsSync(productEntry)) {
    throw new Error(`missing ${productEntry}; run pnpm run build:ts first`);
}
const addon = join(platformDir, `mysql.${platform}.node`);
if (!existsSync(addon)) {
    throw new Error(`missing ${addon}; run node scripts/build-napi.mjs first`);
}

const pnpm = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm';
const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';

const nodeDir = dirname(process.execPath);
const pnpmEntry = join(nodeDir, 'node_modules', 'corepack', 'dist', 'pnpm.js');
const npmEntry = join(nodeDir, 'node_modules', 'npm', 'bin', 'npm-cli.js');

function run(tool, entry, args, options) {
    if (existsSync(entry)) {
        execFileSync(process.execPath, [entry, ...args], options);
        return;
    }
    // Node 24 refuses to spawn `.cmd` shims without a shell on Windows.
    execFileSync(tool, args, { ...options, shell: process.platform === 'win32' });
}

const tempRoot = mkdtempSync(join(tmpdir(), 'mysql-install-'));
const tarballs = join(tempRoot, 'tarballs');
mkdirSync(tarballs, { recursive: true });

function pack(directory) {
    run(pnpm, pnpmEntry, ['pack', '--pack-destination', tarballs], { cwd: directory, stdio: 'inherit' });
}

pack(productDir);
pack(platformDir);
pack(harnessDir);

const tarball = (name) => `file:./tarballs/${name}`;
const dependencies = {
    '@yydb/mysql': tarball('yydb-mysql-0.0.0.tgz'),
    [`@yydb/mysql-${platform}`]: tarball(`yydb-mysql-${platform}-0.0.0.tgz`),
    '@yydb/mysql-harness': tarball('yydb-mysql-harness-0.0.0.tgz'),
};

writeFileSync(
    join(tempRoot, 'package.json'),
    `${JSON.stringify({ private: true, type: 'module', dependencies }, null, 4)}\n`,
);

run(npm, npmEntry, ['install', '--no-audit', '--no-fund', '--no-optional'], {
    cwd: tempRoot,
    stdio: 'inherit',
});

const installedHarness = join(tempRoot, 'node_modules', '@yydb', 'mysql-harness');
const installedEntry = join(tempRoot, 'node_modules', '@yydb', 'mysql', 'dist', 'index.js');
for (const stage of ['node.mjs', 'api.mjs']) {
    execFileSync(process.execPath, [join(installedHarness, 'stages', stage)], {
        cwd: tempRoot,
        env: {
            ...process.env,
            MYSQL_PRODUCT_ENTRY: installedEntry,
        },
        stdio: 'inherit',
    });
}

try {
    rmSync(tempRoot, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
} catch (error) {
    console.error(`install-state cleanup retried but failed: ${error.message}`);
    process.exitCode = 1;
}
