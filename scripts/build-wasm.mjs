// Builds the wasm-pack artifacts into the browser platform package.
//
// The glue and the `.wasm` binary are CI output: they are ignored by git and
// only `package.json#files` names them. `--no-pack` keeps wasm-pack from
// writing its own package.json over the hand-maintained manifest.
import { execFileSync } from 'node:child_process';
import { existsSync, rmSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const crateDir = join(repo, 'projects', 'mysql.rs', 'mysql-wasm');
const outDir = join(repo, 'projects', 'mysql.ts', 'platforms', 'mysql-unknown-wasm32', 'lib');
const dev = process.argv.includes('--dev');

execFileSync(
    'wasm-pack',
    [
        'build',
        crateDir,
        '--target',
        'web',
        '--out-name',
        'mysql_wasm',
        '--out-dir',
        outDir,
        '--no-pack',
        ...(dev ? ['--dev'] : ['--release']),
    ],
    { cwd: repo, stdio: 'inherit' },
);

// wasm-pack writes `.gitignore` containing `*`, which would drop the generated
// glue and `.wasm` from the packed tarball. The file is local build state only.
const generatedIgnore = join(outDir, '.gitignore');
if (existsSync(generatedIgnore)) {
    rmSync(generatedIgnore);
}

console.log(`wasm package -> ${outDir}`);
