// Resolves the platform addon through the optional dependency, or a sibling
// platform package directory on a plain checkout. `package.json#main` owns the
// `.node` filename — callers never concatenate it.
import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');

export type PlatformKey = 'win32-x64' | 'linux-x64' | 'darwin-x64' | 'darwin-arm64';

/** One decoded classic packet. */
export interface DecodedPacket {
    /** Sequence byte from the 4-byte header. */
    readonly sequence: number;
    /** Payload bytes the header announced. */
    readonly payload: Buffer;
}

/** Primitives exported by the `mysql-napi` binding. */
export interface MysqlNative {
    /** Largest payload one classic packet may carry. */
    maxPacketPayload(): number;
    /** Protocol version announced in the initial handshake. */
    protocolVersion(): number;
    /** Frames one payload with the classic 4-byte header. */
    encodePacket(sequence: number, payload: Buffer): Buffer;
    /** Decodes the first complete packet, or `null` when the input is incomplete. */
    decodePacket(input: Buffer): DecodedPacket | null;
}

/** Platform key for the running process, or `null` when no binding exists. */
export function platformKey(platform = process.platform, arch = process.arch): PlatformKey | null {
    if (platform === 'win32' && arch === 'x64') return 'win32-x64';
    if (platform === 'linux' && arch === 'x64') return 'linux-x64';
    if (platform === 'darwin' && arch === 'x64') return 'darwin-x64';
    if (platform === 'darwin' && arch === 'arm64') return 'darwin-arm64';
    return null;
}

/** Loads the platform binding for the running process. */
export function loadNative(): MysqlNative {
    const key = platformKey();
    if (!key) throw new Error(`MySQL native binding is unavailable for ${process.platform}-${process.arch}`);
    const platformPackage = `@yydb/mysql-${key}`;
    try {
        return require(platformPackage) as MysqlNative;
    } catch (error) {
        const repo = resolve(packageRoot, '..', '..', '..', '..');
        const local = join(repo, 'projects', 'mysql.ts', 'platforms', `mysql-${key}`);
        if (existsSync(join(local, 'package.json'))) return require(local) as MysqlNative;
        throw new Error(
            `cannot resolve ${platformPackage}: install the matching platform package (${(error as Error).message})`,
        );
    }
}
