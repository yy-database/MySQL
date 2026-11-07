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

/** Exclusive node lease over one MySQL data directory. */
export interface MysqlLeaseHandle {
    /** Data directory held by this lease. */
    dataDir(): string;
}

/** Local KV executor over the catalog's `.yykv` shards. */
export interface MysqlExecutorHandle {
    readonly _brand?: 'MysqlExecutor';
}

/** Persisted SQL schema registry. */
export interface MysqlSchemaHandle {
    /** Current schema revision, bumped by every applied DDL. */
    revision(): number;
}

/** Loopback MySQL listener owned by the upstream gateway. */
export interface MysqlServiceHandle {
    /** Bound address, including the assigned port when started with `0`. */
    address(): string;
    /** Whether the accept worker has exited. */
    isFinished(): boolean;
    /** Stops the listener and waits for its worker to exit. */
    stop(): void;
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
    /** Publishes single-node shard routing into a `.yyds` catalog. */
    mysqlPublishRouting(catalogPath: string, epoch: number, shardIds: string[]): void;
    /** Acquires the exclusive lease for one data directory. */
    MysqlLease: new (dataDir: string, clusterId: string, nodeId: string) => MysqlLeaseHandle;
    /** Opens the KV executor from published `.yyds` routing. */
    MysqlExecutor: { open(dataDir: string, catalogPath: string): MysqlExecutorHandle };
    /** Opens the persisted SQL schema registry. */
    MysqlSchema: { open(path: string): MysqlSchemaHandle };
    /** Starts the loopback listener on `127.0.0.1:<port>`. */
    MysqlService: {
        start(
            lease: MysqlLeaseHandle,
            executor: MysqlExecutorHandle,
            schema: MysqlSchemaHandle,
            port: number,
        ): MysqlServiceHandle;
    };
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
    // A workspace checkout wins over the package manager's installed copy: pnpm
    // materialises the platform package under `node_modules/.pnpm` at install
    // time, so a freshly built addon only lands in the workspace directory.
    // Installed consumers have no workspace directory and fall through.
    const repo = resolve(packageRoot, '..', '..', '..', '..');
    const local = join(repo, 'projects', 'mysql.ts', 'platforms', `mysql-${key}`);
    if (existsSync(join(local, 'package.json'))) return require(local) as MysqlNative;
    try {
        return require(platformPackage) as MysqlNative;
    } catch (error) {
        throw new Error(
            `cannot resolve ${platformPackage}: install the matching platform package (${(error as Error).message})`,
        );
    }
}
