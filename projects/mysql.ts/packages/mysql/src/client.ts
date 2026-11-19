/// <reference lib="esnext.disposable" />
// Remote client owner. Construction validates and normalizes the address;
// `connect()` opens a real TCP session through the native `MysqlClientSession`
// handle: the startup handshake and the mysql_native_password / caching_sha2
// authentication run upstream, never in TypeScript. `query()` and `execute()`
// dispatch over that session — text protocol without parameters, prepared
// binary protocol with them — and the native binding restores typed values
// from the declared column metadata. Session borrowing, transactions and
// per-request controls stay capability gaps because the exported handle does
// not provide them: no second protocol state machine, pool or timeout
// scheduler grows here.

import { errorText, mysqlCapabilityGapError, mysqlNativeError, mysqlStateError } from './errors.ts';
import {
    loadNative,
    type MysqlClientResultHandle,
    type MysqlClientSessionHandle,
    type MysqlNative,
} from './native.ts';
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
    MySQLColumn,
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
    #session: MysqlClientSessionHandle | undefined;

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
     * Performs the startup handshake and authentication through the native
     * client handle. Concurrent calls join the same operation; a closed client
     * never reconnects — build a new object instead.
     */
    async connect(): Promise<void> {
        if (this.#state === 'ready') return;
        if (this.#state === 'connecting' && this.#operation !== undefined) return this.#operation;
        if (this.#state === 'closed' || this.#state === 'closing') {
            throw mysqlStateError('invalid_state', `connect() cannot revive a closing or closed MySQLClient (state ${this.#state})`, 'connect');
        }
        this.#state = 'connecting';
        const operation = this.#openSession();
        this.#operation = operation;
        try {
            await operation;
        } finally {
            if (this.#operation === operation) this.#operation = undefined;
        }
    }

    /** Runs one statement that returns a row set. */
    async query<R = MySQLRow>(sql: string, params?: readonly MySQLValue[], options?: MySQLQueryOptions): Promise<MySQLResult<R>> {
        this.#requireReady('query');
        validateSql(sql);
        validateParams(params);
        validateRequestOptions(options);
        this.#rejectRequestControls(options);
        const handle = await this.#run(sql, params);
        const columns: MySQLColumn[] = handle.columns.map((column) => ({
            name: column.name,
            typeCode: column.typeCode,
            unsigned: column.unsigned,
            binary: column.binary,
        }));
        const rows =
            options?.arrayRows === true
                ? handle.rows.map((row) => [...row])
                : handle.rows.map((row) => {
                      const record: Record<string, MySQLValue> = {};
                      // Duplicate column labels collapse into one key and the
                      // later column wins; callers that need both pass
                      // `arrayRows`.
                      columns.forEach((column, index) => {
                          record[column.name] = row[index] ?? null;
                      });
                      return record;
                  });
        return { rows: rows as unknown as readonly R[], columns };
    }

    /** Runs one statement that writes. */
    async execute(sql: string, params?: readonly MySQLValue[], options?: RequestOptions): Promise<MySQLWriteResult> {
        this.#requireReady('execute');
        validateSql(sql);
        validateParams(params);
        validateRequestOptions(options);
        this.#rejectRequestControls(options);
        const handle = await this.#run(sql, params);
        return {
            affectedRows: handle.affectedRows,
            lastInsertId: handle.lastInsertId === 0n ? null : handle.lastInsertId,
            // The upstream client does not surface OK-packet warnings yet, so
            // the list stays empty rather than inventing entries.
            warnings: [],
        };
    }

    /** Borrows one fixed physical connection for the callback. */
    async withSession<T>(fn: (session: MySQLSession) => Promise<T>, options?: RequestOptions): Promise<T> {
        this.#requireReady('withSession');
        validateCallback(fn, 'withSession');
        validateRequestOptions(options);
        throw mysqlCapabilityGapError(
            'client session borrowing and pooling (the native handle exposes one fixed session, and no pool primitive is exported yet)',
            'execute',
        );
    }

    /** Runs one callback inside a transaction whose commit is decided natively. */
    async transaction<T>(fn: (tx: MySQLTransaction) => Promise<T>, options?: MySQLTransactionOptions): Promise<T> {
        this.#requireReady('transaction');
        validateCallback(fn, 'transaction');
        validateTransactionOptions(options);
        throw mysqlCapabilityGapError(
            'client transaction state machinery (native begin/commit/rollback over the session is not exported yet)',
            'execute',
        );
    }

    /**
     * Closes this client's connections and derived resources. It never closes
     * the remote server. Repeating close succeeds.
     */
    async close(): Promise<void> {
        if (this.#state === 'closed') return;
        if (this.#state === 'closing' && this.#operation !== undefined) return this.#operation;
        this.#state = 'closing';
        const operation = Promise.resolve().then(() => {
            // Dropping the native session closes the TCP socket; there is no
            // network round trip to await.
            this.#session?.close();
            this.#session = undefined;
        });
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

    async #openSession(): Promise<void> {
        let native: MysqlNative;
        try {
            native = loadNative();
        } catch (error) {
            this.#state = 'failed';
            throw mysqlNativeError('native_load_failed', errorText(error), error, 'connect', 'not-sent', 'never');
        }
        const { endpoint } = this.#settings;
        let session: MysqlClientSessionHandle;
        try {
            session = await native.mysqlClientConnect({
                host: endpoint.host,
                port: endpoint.port,
                user: this.#settings.auth?.username ?? 'root',
                password: this.#settings.auth?.password ?? '',
                ...(this.#settings.database === undefined ? {} : { database: this.#settings.database }),
                ...(this.#settings.connectTimeoutMs === undefined ? {} : { timeoutMs: this.#settings.connectTimeoutMs }),
            });
        } catch (error) {
            this.#state = 'failed';
            throw mysqlNativeError(
                'native_connect_failed',
                `the client handshake against ${endpoint.host}:${endpoint.port} failed: ${errorText(error)}`,
                error,
                'connect',
            );
        }
        this.#session = session;
        this.#state = 'ready';
    }

    async #run(sql: string, params: readonly MySQLValue[] | undefined): Promise<MysqlClientResultHandle> {
        const session = this.#session;
        if (session === undefined) {
            throw mysqlStateError('client_not_connected', 'the MySQLClient session is no longer available', 'execute');
        }
        try {
            return params === undefined ? await session.query(sql) : await session.queryWithParams(sql, shapeParams(params));
        } catch (error) {
            // The statement was written to the socket; whether the server
            // applied it before the failure cannot be proven from here.
            throw mysqlNativeError(
                'native_execute_failed',
                `the client statement failed: ${errorText(error)}`,
                error,
                'execute',
                'unknown',
            );
        }
    }

    #rejectRequestControls(options: RequestOptions | undefined): void {
        if (options?.timeoutMs !== undefined || options?.signal !== undefined) {
            throw mysqlCapabilityGapError('per-request timeout and abort signal on the client session', 'configuration');
        }
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

/**
 * Shapes author-facing parameters for the native prepared path. Numbers and
 * bigints travel as decimal text and raw octets stay octets: the server
 * coerces against the target column instead of this file guessing types.
 */
function shapeParams(params: readonly MySQLValue[]): (string | Uint8Array | null)[] {
    return params.map((value) => {
        if (value === null || typeof value === 'string' || value instanceof Uint8Array) return value;
        return String(value);
    });
}
