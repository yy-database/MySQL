// Install-state acceptance for the Python product.
//
// Builds the wheel from source with maturin, creates a clean virtualenv under a
// system temporary directory, installs the wheel plus the real drivers and runs
// every declarative case from inside the virtualenv. The product is never
// imported from the workspace, and this implementation never stands in for a
// real MySQL server or a real driver.

import { spawnSync } from 'node:child_process';
import { mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const harness = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repo = resolve(harness, '..', '..', '..', '..');
const packageDir = join(repo, 'projects', 'mysql.py', 'packages', 'mysql');
const casesDir = join(harness, 'cases');
const runner = join(harness, 'stages', 'python', 'run.py');

const python = process.env.PYTHON ?? (process.platform === 'win32' ? 'python' : 'python3');

// CI installs the `maturin` console script through pip. A local install may
// only have the module, so fall back to `python -m maturin`.
function resolveMaturin() {
    if (process.env.MATURIN) return { command: process.env.MATURIN, prefix: [] };
    const probe = spawnSync('maturin', ['--version'], { encoding: 'utf8' });
    if (!probe.error) return { command: 'maturin', prefix: [] };
    return { command: python, prefix: ['-m', 'maturin'] };
}

const maturin = resolveMaturin();

const failures = [];

function run(command, args, options = {}) {
    const result = spawnSync(command, args, { encoding: 'utf8', ...options });
    if (result.error) throw new Error(`${command} could not start: ${result.error.message}`);
    if (result.status !== 0) {
        const detail = (result.stderr ?? '').trim();
        throw new Error(`${command} ${args.join(' ')} exited with ${result.status}${detail ? `: ${detail}` : ''}`);
    }
    return result;
}

const root = mkdtempSync(join(tmpdir(), 'mysql-python-'));

try {
    const wheels = join(root, 'wheels');
    const built = run(maturin.command, [...maturin.prefix, 'build', '--release', '--out', wheels], { cwd: packageDir });
    if (built.stdout.length > 0) process.stdout.write(built.stdout);
    const wheel = readdirSync(wheels).find((name) => name.endsWith('.whl'));
    if (!wheel) throw new Error(`maturin produced no wheel in ${wheels}`);

    const venv = join(root, 'venv');
    run(python, ['-m', 'venv', venv], { stdio: 'inherit' });
    const venvPython = process.platform === 'win32' ? join(venv, 'Scripts', 'python.exe') : join(venv, 'bin', 'python');

    // Both real drivers are installed first: `mysql-connector-python` also
    // ships a top-level `mysql` package, so the product wheel goes last and
    // wins the `mysql/__init__.py` collision. A failure here is reported as
    // driver unavailable, never replaced by comparing this implementation
    // against itself.
    try {
        run(venvPython, ['-m', 'pip', 'install', '--quiet', '--disable-pip-version-check', 'pymysql', 'mysql-connector-python']);
        run(venvPython, ['-m', 'pip', 'install', '--quiet', '--disable-pip-version-check', join(wheels, wheel)]);
    } catch (error) {
        throw new Error(`driver unavailable: ${error.message}`);
    }

    const result = spawnSync(venvPython, [runner, '--cases', casesDir, '--root', join(root, 'cases')], {
        stdio: 'inherit',
    });
    if (result.error) throw new Error(`python runner could not start: ${result.error.message}`);
    if (result.status === 3) throw new Error('driver unavailable: see the runner output above');
    if (result.status !== 0) throw new Error(`python runner exited with ${result.status}`);
} catch (error) {
    failures.push(error.message);
    console.error(`FAIL python stage: ${error.message}`);
} finally {
    try {
        rmSync(root, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
    } catch (error) {
        console.error(`cleanup retried but failed: ${error.message}`);
        failures.push(`cleanup: ${error.message}`);
    }
}

if (failures.length > 0) {
    for (const failure of failures) console.error(`FAILED ${failure}`);
    process.exit(1);
}
