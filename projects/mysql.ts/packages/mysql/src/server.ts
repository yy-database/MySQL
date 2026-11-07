// Composes the upstream MySQL gateway with a `.yyds` catalog, a `.yykv` shard
// and a persisted SQL schema registry. Every protocol byte comes from
// `database.server::yyds::mysql`; this file only orders the primitives.

import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

import { loadNative } from './native.ts';

/** Options for {@link startMysqlServer}. */
export interface MysqlServerOptions {
    /** Directory holding `catalog.yyds`, `<shard>.yykv` and `sql-schema.v0.json`. */
    dataDir: string;
    /** Loopback port; `0` lets the operating system pick one. Defaults to `0`. */
    port?: number;
    /** Cluster identity written to the node lease. Defaults to `mysql-local`. */
    clusterId?: string;
    /** Node identity written to the node lease. Defaults to `mysql-node-0`. */
    nodeId?: string;
}

/** A running loopback MySQL listener. */
export interface MysqlServer {
    /** Bound `host:port`, including the port the operating system assigned. */
    readonly address: string;
    /** Numeric bound port. */
    readonly port: number;
    readonly dataDir: string;
    readonly catalogPath: string;
    readonly schemaPath: string;
    /** Stops the listener and releases the directory lease. */
    stop(): void;
}

/** Single-node shard id used by a MySQL data directory. */
export const LOCAL_SHARD_ID = 'local-0';

/**
 * Starts a MySQL listener over `<dataDir>`.
 *
 * The call order matters: routing is published into `.yyds` first, the KV
 * executor opens the `.yykv` shard it names, the schema registry opens its
 * persisted file, and only then does the upstream gateway start listening.
 */
export function startMysqlServer(options: MysqlServerOptions): MysqlServer {
    const native = loadNative();
    const dataDir = options.dataDir;
    mkdirSync(dataDir, { recursive: true });

    const catalogPath = join(dataDir, 'catalog.yyds');
    const schemaPath = join(dataDir, 'sql-schema.v0.json');

    native.mysqlPublishRouting(catalogPath, 1, [LOCAL_SHARD_ID]);
    const lease = new native.MysqlLease(
        dataDir,
        options.clusterId ?? 'mysql-local',
        // The lease pins one identity per data directory. A per-process id
        // would make the second process after a restart fail the identity check.
        options.nodeId ?? 'mysql-node-0',
    );
    const executor = native.MysqlExecutor.open(dataDir, catalogPath);
    const schema = native.MysqlSchema.open(schemaPath);
    const service = native.MysqlService.start(lease, executor, schema, options.port ?? 0);

    const address = service.address();
    const port = Number(address.slice(address.lastIndexOf(':') + 1));
    let stopped = false;

    return {
        address,
        port,
        dataDir,
        catalogPath,
        schemaPath,
        stop() {
            if (stopped) return;
            stopped = true;
            service.stop();
        },
    };
}
