/// <reference lib="esnext.disposable" />
// Local engine owner. Construction validates configuration; `open()` brings the
// storage up through the exported native handles in the order the gateway
// requires (routing published into `.yyds`, then the exclusive lease, the KV
// executor, the persisted schema registry). `listen()` is the only call that
// opens the network, and closing a listener never closes the engine.
//
// SQL execution is NOT reimplemented here: the exported native binding carries
// no in-process session handle yet, so the execution surface fails through the
// error envelope with `native_capability_missing` instead of running a second
// engine, a second codec or a loopback detour in TypeScript.

import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

import {
    errorText,
    mysqlCapabilityGapError,
    mysqlNativeError,
    mysqlStateError,
} from './errors.ts';
import {
    loadNative,
    type MysqlExecutorHandle,
    type MysqlLeaseHandle,
    type MysqlNative,
    type MysqlSchemaHandle,
    type MysqlServiceHandle,
} from './native.ts';
import {
    resolveMySQLOptions,
    resolveMySQLListenOptions,
    validateCallback,
    validateParams,
    validateRequestOptions,
    validateSql,
    validateTransactionOptions,
    type MySQLOptions,
    type MySQLListenOptions,
    type RequestOptions,
    type ResolvedMySQLOptions,
    type TcpEndpoint,
} from './options.ts';
import { CATALOG_FILE, LOCAL_SHARD_ID, SCHEMA_FILE } from './server.ts';
import type {
    MySQLQueryOptions,
    MySQLResult,
    MySQLRow,
    MySQLSession,
    MySQLTransaction,
    MySQLTransactionOptions,
    MySQLValue,
    MySQLWriteResult,
} from './session.ts';

/** Lifecycle of the local engine owner. */
export type LocalState = 'new' | 'opening' | 'open' | 'closing' | 'closed' | 'failed';

/** One loopback listener derived from an open {@link MySQL} instance. */
export interface MySQLListener {
    /** Bound loopback endpoint, including the port the operating system assigned. */
    readonly endpoint: TcpEndpoint;
    /** Bound `host:port` text. */
    readonly address: string;
    /** Stops this listener only; the engine stays open. Idempotent. */
    close(): Promise<void>;
    [Symbol.asyncDispose](): Promise<void>;
}

class MySQLListenerHandle implements MySQLListener {
    #service: MysqlServiceHandle | undefined;
    readonly #owner: Set<MySQLListenerHandle>;

    constructor(
        service: MysqlServiceHandle,
        readonly address: string,
        readonly endpoint: TcpEndpoint,
        owner: Set<MySQLListenerHandle>,
    ) {
        this.#service = service;
        this.#owner = owner;
    }

    async close(): Promise<void> {
        const service = this.#service;
        if (service === undefined) return;
        this.#service = undefined;
        this.#owner.delete(this);
        try {
            service.stop();
        } catch (error) {
            throw mysqlNativeError(
                'native_close_failed',
                `stopping the MySQL listener at ${this.address} failed: ${errorText(error)}`,
                error,
                'close',
                'unknown',
            );
        }
    }

    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }
}

/** Local MySQL engine: owns the data directory lease and derives listeners. */
export class MySQL {
    readonly #config: ResolvedMySQLOptions;
    #state: LocalState = 'new';
    #operation: Promise<void> | undefined;
    #native: MysqlNative | undefined;
    #lease: MysqlLeaseHandle | undefined;
    #executor: MysqlExecutorHandle | undefined;
    #schema: MysqlSchemaHandle | undefined;
    readonly #listeners = new Set<MySQLListenerHandle>();

    constructor(options: MySQLOptions = {}) {
        this.#config = resolveMySQLOptions(options);
    }

    /** Current lifecycle state. */
    get state(): LocalState {
        return this.#state;
    }

    /** Configured default database, kept as client/session configuration. */
    get database(): string | undefined {
        return this.#config.database;
    }

    /** Data directory owned by this instance. */
    get storageDirectory(): string {
        return this.#config.storage.directory;
    }

    /**
     * Brings the engine up over `options.storage.directory`. Concurrent calls
     * join the same operation; a closed instance never reopens — build a new
     * object instead.
     */
    async open(): Promise<void> {
        if (this.#state === 'open') return;
        if (this.#state === 'opening' && this.#operation !== undefined) return this.#operation;
        if (this.#state !== 'new' && this.#state !== 'failed') {
            throw mysqlStateError('invalid_state', `open() requires state new or failed (state ${this.#state})`, 'open');
        }
        this.#state = 'opening';
        const operation = this.#bringUpAndMark();
        this.#operation = operation;
        try {
            await operation;
        } finally {
            if (this.#operation === operation) this.#operation = undefined;
        }
    }

    /**
     * Starts one loopback listener over the open engine and returns it as an
     * independent resource. Closing the listener stops accepting and waits for
     * the accept worker; it does not close the engine.
     */
    async listen(options: MySQLListenOptions = {}): Promise<MySQLListener> {
        if (this.#state !== 'open') {
            throw mysqlStateError('invalid_state', `listen() requires an open MySQL instance; call open() first (state ${this.#state})`, 'open');
        }
        const { port } = resolveMySQLListenOptions(options);
        const native = this.#native;
        const lease = this.#lease;
        const executor = this.#executor;
        const schema = this.#schema;
        if (native === undefined || lease === undefined || executor === undefined || schema === undefined) {
            throw mysqlStateError('invalid_state', `listen() requires an open MySQL instance (state ${this.#state})`, 'open');
        }
        let service: MysqlServiceHandle;
        try {
            service = native.MysqlService.start(lease, executor, schema, port);
        } catch (error) {
            throw mysqlNativeError('native_listen_failed', `starting the MySQL listener failed: ${errorText(error)}`, error, 'open');
        }
        const address = service.address();
        const boundPort = Number(address.slice(address.lastIndexOf(':') + 1));
        const listener = new MySQLListenerHandle(service, address, { host: '127.0.0.1', port: boundPort }, this.#listeners);
        this.#listeners.add(listener);
        return listener;
    }

    /** Runs one statement that returns a row set. */
    async query<R = MySQLRow>(sql: string, params?: readonly MySQLValue[], options?: MySQLQueryOptions): Promise<MySQLResult<R>> {
        validateSql(sql);
        validateParams(params);
        validateRequestOptions(options);
        this.#requireOpen('query');
        throw mysqlCapabilityGapError('in-process SQL execution over the local executor (a native session handle)', 'execute');
    }

    /** Runs one statement that writes. */
    async execute(sql: string, params?: readonly MySQLValue[], options?: RequestOptions): Promise<MySQLWriteResult> {
        validateSql(sql);
        validateParams(params);
        validateRequestOptions(options);
        this.#requireOpen('execute');
        throw mysqlCapabilityGapError('in-process SQL execution over the local executor (a native session handle)', 'execute');
    }

    /** Borrows one fixed session for the callback. */
    async withSession<T>(fn: (session: MySQLSession) => Promise<T>, options?: RequestOptions): Promise<T> {
        validateCallback(fn, 'withSession');
        validateRequestOptions(options);
        this.#requireOpen('withSession');
        throw mysqlCapabilityGapError('session borrowing over the local executor (a native session handle)', 'execute');
    }

    /** Runs one callback inside a transaction whose commit is decided natively. */
    async transaction<T>(fn: (tx: MySQLTransaction) => Promise<T>, options?: MySQLTransactionOptions): Promise<T> {
        validateCallback(fn, 'transaction');
        validateTransactionOptions(options);
        this.#requireOpen('transaction');
        throw mysqlCapabilityGapError(
            'transaction state machinery over the local executor (native begin/commit/rollback)',
            'execute',
        );
    }

    /**
     * Stops every derived listener and releases the engine. Repeating close on
     * a closed instance succeeds.
     */
    async close(): Promise<void> {
        if (this.#state === 'closed') return;
        if (this.#state === 'closing' && this.#operation !== undefined) return this.#operation;
        if (this.#state === 'opening' && this.#operation !== undefined) {
            try {
                await this.#operation;
            } catch {
                // Bring-up failed: the native handles were never all live, so
                // there is nothing beyond partial drops to unwind.
            }
        }
        this.#state = 'closing';
        const operation = this.#shutDown();
        this.#operation = operation;
        try {
            await operation;
        } finally {
            if (this.#operation === operation) this.#operation = undefined;
            this.#state = 'closed';
        }
    }

    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    #releaseHandles(): void {
        this.#native = undefined;
        this.#lease = undefined;
        this.#executor = undefined;
        this.#schema = undefined;
    }

    async #bringUpAndMark(): Promise<void> {
        try {
            await this.#bringUp();
        } catch (error) {
            this.#releaseHandles();
            this.#state = 'failed';
            throw error;
        }
        this.#state = 'open';
    }

    async #bringUp(): Promise<void> {
        let native: MysqlNative;
        try {
            native = loadNative();
        } catch (error) {
            throw mysqlNativeError('native_load_failed', errorText(error), error, 'open', 'not-sent', 'never');
        }
        const dataDir = this.#config.storage.directory;
        const catalogPath = join(dataDir, CATALOG_FILE);
        const schemaPath = join(dataDir, SCHEMA_FILE);
        try {
            mkdirSync(dataDir, { recursive: true });
            native.mysqlPublishRouting(catalogPath, 1, [LOCAL_SHARD_ID]);
            this.#lease = new native.MysqlLease(dataDir, 'mysql-local', 'mysql-node-0');
            this.#executor = native.MysqlExecutor.open(dataDir, catalogPath);
            this.#schema = native.MysqlSchema.open(schemaPath);
        } catch (error) {
            throw mysqlNativeError(
                'native_open_failed',
                `cannot open the MySQL local engine at ${dataDir}: ${errorText(error)}`,
                error,
                'open',
            );
        }
        this.#native = native;
    }

    async #shutDown(): Promise<void> {
        const failures: unknown[] = [];
        for (const listener of [...this.#listeners]) {
            try {
                await listener.close();
            } catch (error) {
                failures.push(error);
            }
        }
        this.#listeners.clear();
        // The lease, executor and schema handles expose no release call: the
        // native layer drops them when the last reference goes away.
        this.#releaseHandles();
        if (failures.length > 0) {
            throw mysqlNativeError(
                'native_close_failed',
                `closing the MySQL local engine left ${failures.length} listener(s) failing to stop: ${errorText(failures[0])}`,
                failures[0],
                'close',
                'unknown',
            );
        }
    }

    #requireOpen(call: string): void {
        if (this.#state === 'open') return;
        throw mysqlStateError('invalid_state', `${call}() requires an open MySQL instance (state ${this.#state})`, 'execute');
    }
}
