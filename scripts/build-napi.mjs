// Builds `mysql-napi` and drops the addon into the matching platform package.
import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const crateDir = join(repo, 'projects', 'mysql.rs');
const platform = `${process.platform}-${process.arch}`;
const packageDir = join(repo, 'projects', 'mysql.ts', 'platforms', `mysql-${platform}`);
const nodeName = `mysql.${platform}.node`;

const artifact = {
    win32: { name: 'mysql_napi.dll' },
    darwin: { name: 'libmysql_napi.dylib' },
    linux: { name: 'libmysql_napi.so' },
}[process.platform];

if (!artifact) {
    throw new Error(`unsupported platform ${process.platform}`);
}

const release = process.argv.includes('--release');
execFileSync(
    'cargo',
    ['build', '-p', 'mysql-napi', ...(release ? ['--release'] : [])],
    { cwd: crateDir, stdio: 'inherit' },
);

const profile = release ? 'release' : 'debug';
const targetDirectory = resolve(crateDir, process.env.CARGO_TARGET_DIR ?? 'target');
const source = join(targetDirectory, profile, artifact.name);
if (!existsSync(source)) {
    throw new Error(`missing build artifact ${source}`);
}

const dest = join(packageDir, nodeName);
copyFileSync(source, dest);
console.log(`napi addon -> ${dest}`);
