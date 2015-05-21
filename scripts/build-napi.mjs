import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const rustWorkspace = join(repositoryRoot, 'projects', 'mysql.rs');
const release = process.argv.includes('--release');
const profile = release ? 'release' : 'debug';

const platforms = {
    'win32-x64': { package: 'mysql-win32-x64', file: 'mysql-win32-x64-msvc.node', artifact: 'mysql_napi.dll' },
    'linux-x64': { package: 'mysql-linux-x64', file: 'mysql-linux-x64-gnu.node', artifact: 'libmysql_napi.so' },
    'darwin-x64': { package: 'mysql-darwin-x64', file: 'mysql-darwin-x64.node', artifact: 'libmysql_napi.dylib' },
    'darwin-arm64': { package: 'mysql-darwin-arm64', file: 'mysql-darwin-arm64.node', artifact: 'libmysql_napi.dylib' },
};

const key = `${process.platform}-${process.arch}`;
const platform = platforms[key];
if (!platform) {
    throw new Error(`no MySQL platform package for ${key}`);
}

execFileSync('cargo', ['build', '--manifest-path', join(rustWorkspace, 'Cargo.toml'), '-p', 'mysql-napi', ...(release ? ['--release'] : [])], {
    cwd: rustWorkspace,
    stdio: 'inherit',
});

const targetDirectory = process.env.CARGO_TARGET_DIR ?? join(rustWorkspace, 'target');
const destination = join(repositoryRoot, 'projects', 'mysql.ts', 'platforms', platform.package, 'lib');
mkdirSync(destination, { recursive: true });
copyFileSync(join(targetDirectory, profile, platform.artifact), join(destination, platform.file));
console.log(`copied ${platform.file} into ${platform.package}/lib`);
