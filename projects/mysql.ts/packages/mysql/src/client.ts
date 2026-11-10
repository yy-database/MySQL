/// <reference lib="esnext.disposable" />
// Remote client owner. Construction validates and normalizes the address;
// `connect()` is the place where the exported protocol primitives would run
// the startup handshake and password authentication. The current native
// binding exports packet framing only — no handshake parser and no
// mysql_native_password / caching_sha2 client handle — so `connect()` fails
// through the error envelope with `native_capability_missing` instead of
// growing a second protocol state machine in TypeScript.

import { mysqlCapabilityGapError, mysqlStateError } from './errors.ts';
import {
    resolveMySQLClientOptions,
    validateCallback,
    validateParams,
    validateRequestOptions,
    validateSql,
    validateTransactionOptions,
    type MySQLClientOptions,
    type RequestOptions,
    type ResolvedMySQLClientOptions,
    type TcpEndpoint,
} from './options.ts';
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

/** Lifecycle of the remote client. */
export type ClientState = 'new' | 'connecting' | 'ready' | 'reconnecting' | 'closing' | 'closed' | 'failed';

/** Client over one remote MySQL endpoint. Owns its connections, never a server. */
export class MySQLClient {
    readonly #settings: ResolvedMySQLClientOptions;
    #state: ClientState = 'new';
    #operation: Promise<void> | undefined;

    constructor(options: MySQLClientOptions) {
        this.#settings = resolveMySQLClientOptions(options);
    }

    /** Current lifecycle state. */
    get state(): ClientState {
        return this.#state;
    }

    /** Resolved endpoint this client would connect to. */
    get endpoint(): TcpEndpoint {
        return { ...this.#settings.endpoint };
    }

    /** Configured default database, if any. */
    get database(): string | undefined {
        return this.#settings.database;
    }

    /**
     * Performs the startup handshake and authentication through the protocol
     * primitives. Concurrent calls join the same operation; a closed client
     * never reconnects — build a new object instead.
     */
    async connect(): Promise<void> {
        if (this.#state === 'ready') return;
        if (this.#state === 'connecting' && this.#operation !== undefined) return this.#operation;
        if (this.#state === 'closed' || this.#state === 'closing') {
            throw mysqlStateError('invalid_state', `connect() cannot revive a closing or closed MySQLClient (state ${this.#state})`, 'connect');
        }
        throw mysqlCapabilityGapError(
            'the client startup handshake and password authentication (mysql_native_password / caching_sha2)',
            'connect',
        );
    }

    /** Runs one statement that returns a row set. */
    async query<R = MySQLRow>(sql: string, params?: readonly MySQLValue[], options?: MySQLQueryOptions): Promise<MySQLResult<R>> {
        this.#requireReady('query');
        validateSql(sql);
        validateParams(params);
        validateRequestOptions(options);
        throw mysqlCapabilityGapError('client statement execution over a connected session', 'execute');
    }

    /** Runs one statement that writes. */
    async execute(sql: string, params?: readonly MySQLValue[], options?: RequestOptions): Promise<MySQLWriteResult> {
        this.#requireReady('execute');
        validateSql(sql);
        validateParams(params);
        validateRequestOptions(options);
        throw mysqlCapabilityGapError('client statement execution over a connected session', 'execute');
    }

    /** Borrows one fixed physical connection for the callback. */
    async withSession<T>(fn: (session: MySQLSession) => Promise<T>, options?: RequestOptions): Promise<T> {
        this.#requireReady('withSession');
        validateCallback(fn, 'withSession');
        validateRequestOptions(options);
        throw mysqlCapabilityGapError('client session borrowing over a connected pool', 'execute');
    }

    /** Runs one callback inside a transaction whose commit is decided natively. */
    async transaction<T>(fn: (tx: MySQLTransaction) => Promise<T>, options?: MySQLTransactionOptions): Promise<T> {
        this.#requireReady('transaction');
        validateCallback(fn, 'transaction');
        validateTransactionOptions(options);
        throw mysqlCapabilityGapError('client transaction state machinery over a connected session', 'execute');
    }

    /**
     * Closes this client's connections and derived resources. It never closes
     * the remote server. Repeating close succeeds.
     */
    async close(): Promise<void> {
        if (this.#state === 'closed') return;
        if (this.#state === 'closing' && this.#operation !== undefined) return this.#operation;
        this.#state = 'closing';
        const operation = Promise.resolve();
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

    #requireReady(call: string): void {
        if (this.#state === 'ready') return;
        throw mysqlStateError(
            'client_not_connected',
            `${call}() requires a connected MySQLClient; call connect() first (state ${this.#state})`,
            'execute',
        );
    }
}
