import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);

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

/** Native binding filename inside each `@yydb/mysql-<platform>` package `lib/`. */
export function nativeBindingName(key: PlatformKey): string {
    switch (key) {
        case 'win32-x64':
            return 'mysql-win32-x64-msvc.node';
        case 'linux-x64':
            return 'mysql-linux-x64-gnu.node';
        case 'darwin-x64':
            return 'mysql-darwin-x64.node';
        case 'darwin-arm64':
            return 'mysql-darwin-arm64.node';
    }
}

/** Loads the platform binding for the running process. */
export function loadNative(): MysqlNative {
    const key = platformKey();
    if (!key) throw new Error(`MySQL native binding is unavailable for ${process.platform}-${process.arch}`);
    const loaded: { default?: MysqlNative } | MysqlNative = require(`@yydb/mysql-${key}`);
    return ('default' in loaded && loaded.default ? loaded.default : loaded) as MysqlNative;
}
