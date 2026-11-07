// Verifies the browser wasm platform package against its own manifest.
//
// Two properties are checked, because a manifest that merely parses is not
// evidence: every path declared in `main` / `types` / `files` must exist, and
// the declared entry must instantiate its declared `.wasm` and answer a real
// call. The existence check is then negatively proven on a mutated manifest so
// a renamed artifact cannot slip through as a green run.
import { existsSync, readFileSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const packageDir = resolve(
    process.argv[2] ?? join('projects', 'mysql.ts', 'platforms', 'mysql-unknown-wasm32'),
);
const manifest = JSON.parse(readFileSync(join(packageDir, 'package.json'), 'utf8'));

/** Every file path the manifest promises to ship. */
function declaredPaths(pkg) {
    const paths = new Set();
    for (const field of [pkg.main, pkg.types]) {
        if (field) paths.add(field);
    }
    for (const file of pkg.files ?? []) {
        paths.add(file);
    }
    return [...paths];
}

/** Throws when any declared path is not a real file on disk. */
function assertDeclaredFiles(pkg, dir) {
    for (const relative of declaredPaths(pkg)) {
        const absolute = join(dir, relative);
        if (!existsSync(absolute) || !statSync(absolute).isFile()) {
            throw new Error(`declared file is missing: ${relative}`);
        }
    }
}

assertDeclaredFiles(manifest, packageDir);

// Negative proof: a manifest naming a nonexistent artifact must be rejected,
// otherwise the check above is green for the wrong reason.
const broken = { ...manifest, main: 'mysql_wasm_bg.wasm.missing' };
let rejected = false;
try {
    assertDeclaredFiles(broken, packageDir);
} catch {
    rejected = true;
}
if (!rejected) {
    throw new Error('declaration check accepted a manifest name that does not exist');
}

const entry = manifest.exports?.['.']?.import;
if (entry !== manifest.main) {
    throw new Error(`exports["."].import must equal ${manifest.main}, got ${entry}`);
}

const wasmFile = (manifest.files ?? []).find((file) => file.endsWith('.wasm'));
if (!wasmFile) {
    throw new Error('package.json#files declares no .wasm asset');
}

// `--target web` glue is an ES module whose default export is the async init.
const glue = await import(pathToFileURL(join(packageDir, manifest.main)).href);
if (typeof glue.default !== 'function') {
    throw new Error(`${manifest.main} has no default init export`);
}
await glue.default({ module_or_path: readFileSync(join(packageDir, wasmFile)) });

const version = glue.protocol_version();
if (version !== 10) {
    throw new Error(`protocol_version() returned ${version}, expected 10`);
}

const payload = Buffer.from('SELECT 1');
const framed = glue.encode_packet(0, payload);
const packet = glue.decode_packet(framed);
if (!packet || packet.sequence !== 0 || Buffer.from(packet.payload).toString() !== 'SELECT 1') {
    throw new Error('encode_packet/decode_packet did not roundtrip SELECT 1');
}

console.log(
    `wasm package ok: ${manifest.name} protocol=${version} payload=${packet.payload.length} bytes`,
);
