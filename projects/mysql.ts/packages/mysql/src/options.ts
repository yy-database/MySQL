// Constructor option shapes and validation. Construction only stores and
// validates configuration; nothing here touches the native binding. URL
// parsing is configuration shaping, not protocol work: the parsed fields are
// handed to the native client primitive once it exists, and the wire
// handshake itself never happens in TypeScript.

import { mysqlCapabilityGapError, mysqlConfigurationError } from './errors.ts';
import type { MySQLIsolationLevel, MySQLValue } from './session.ts';

/** Directory-backed local storage. */
export interface LocalStorage {
    readonly directory: string;
}

/** Internet host and port. */
export interface TcpEndpoint {
    readonly host: string;
    readonly port: number;
}

/** Username and password for password authentication. */
export interface PasswordAuth {
    readonly username: string;
    readonly password: string;
}

/** Per-request control shared by query, execute and scope helpers. */
export interface RequestOptions {
    readonly signal?: AbortSignal;
    readonly timeoutMs?: number;
}

/** Options for the local engine owner `new MySQL(options?)`. */
export interface MySQLOptions {
    readonly storage?: LocalStorage;
    readonly database?: string;
}

/** Validated {@link MySQLOptions} as stored by the constructor. */
export interface ResolvedMySQLOptions {
    readonly storage: LocalStorage;
    readonly database?: string;
}

/** Options for `mysql.listen(options?)`. */
export interface MySQLListenOptions {
    readonly endpoint?: TcpEndpoint;
    readonly authentication?: PasswordAuth;
}

/** Client settings that are independent of how the address is spelled. */
export interface MySQLClientSettings {
    readonly database?: string;
    readonly auth?: PasswordAuth;
    readonly charset?: string;
    readonly connectTimeoutMs?: number;
}

/** Options for the remote client `new MySQLClient(options)`: `url` and `endpoint` are exclusive. */
export type MySQLClientOptions = (
    | { readonly url: string; readonly endpoint?: never }
    | { readonly url?: never; readonly endpoint: TcpEndpoint }
) &
    MySQLClientSettings;

/** Validated {@link MySQLClientOptions} as stored by the constructor. */
export interface ResolvedMySQLClientOptions {
    readonly endpoint: TcpEndpoint;
    readonly database?: string;
    readonly auth?: PasswordAuth;
    readonly charset?: string;
    readonly connectTimeoutMs?: number;
}

function nonEmptyString(value: unknown): value is string {
    return typeof value === 'string' && value.trim() !== '';
}

function validateSettings(settings: MySQLClientSettings): void {
    if (settings.database !== undefined && !nonEmptyString(settings.database)) {
        throw mysqlConfigurationError('invalid_database', 'database must be a non-empty string');
    }
    if (settings.charset !== undefined && !nonEmptyString(settings.charset)) {
        throw mysqlConfigurationError('invalid_charset', 'charset must be a non-empty string');
    }
    if (settings.connectTimeoutMs !== undefined && (!Number.isInteger(settings.connectTimeoutMs) || settings.connectTimeoutMs < 0)) {
        throw mysqlConfigurationError('invalid_connect_timeout', 'connectTimeoutMs must be an integer number of milliseconds at least 0');
    }
    if (settings.auth !== undefined) {
        if (!nonEmptyString(settings.auth.username)) {
            throw mysqlConfigurationError('invalid_auth', 'auth.username must be a non-empty string');
        }
        if (typeof settings.auth.password !== 'string') {
            throw mysqlConfigurationError('invalid_auth', 'auth.password must be a string');
        }
    }
}

function validateEndpoint(endpoint: TcpEndpoint, source: string): TcpEndpoint {
    if (!nonEmptyString(endpoint.host)) {
        throw mysqlConfigurationError('invalid_endpoint', `${source} host must be a non-empty string`);
    }
    if (!Number.isInteger(endpoint.port) || endpoint.port < 1 || endpoint.port > 65535) {
        throw mysqlConfigurationError('invalid_endpoint', `${source} port must be an integer between 1 and 65535`);
    }
    return { host: endpoint.host, port: endpoint.port };
}

/** Validates the local constructor options. */
export function resolveMySQLOptions(options: MySQLOptions): ResolvedMySQLOptions {
    if (options.storage === undefined || !nonEmptyString(options.storage.directory)) {
        throw mysqlConfigurationError(
            'storage_directory_required',
            'MySQL requires options.storage.directory: the exported native lease is bound to one data directory and no in-memory storage primitive is exported yet',
        );
    }
    if (options.database !== undefined && !nonEmptyString(options.database)) {
        throw mysqlConfigurationError('invalid_database', 'database must be a non-empty string');
    }
    return { storage: { directory: options.storage.directory }, database: options.database };
}

/**
 * Validates `listen()` options against what the exported listener primitive
 * supports: the native gateway binds loopback only and takes no authentication
 * parameter, so anything else fails in the configuration phase.
 */
export function resolveMySQLListenOptions(options: MySQLListenOptions): { host: string; port: number } {
    if (options.authentication !== undefined) {
        throw mysqlCapabilityGapError('server-side authentication configuration for listen()', 'configuration');
    }
    if (options.endpoint !== undefined) {
        if (options.endpoint.host !== '127.0.0.1' && options.endpoint.host !== 'localhost') {
            throw mysqlConfigurationError(
                'endpoint_host_unsupported',
                `the exported listener binds loopback addresses only, received host ${JSON.stringify(options.endpoint.host)}`,
            );
        }
        if (!Number.isInteger(options.endpoint.port) || options.endpoint.port < 0 || options.endpoint.port > 65535) {
            throw mysqlConfigurationError('invalid_endpoint', 'listen endpoint port must be an integer between 0 and 65535');
        }
    }
    return { host: '127.0.0.1', port: options.endpoint?.port ?? 0 };
}

/** Validates the remote client constructor options. */
export function resolveMySQLClientOptions(options: MySQLClientOptions): ResolvedMySQLClientOptions {
    if (options.url !== undefined && options.endpoint !== undefined) {
        throw mysqlConfigurationError('address_conflict', 'MySQLClient accepts url or endpoint, not both');
    }
    validateSettings(options);

    if (options.url !== undefined) {
        const parsed = parseMySQLUrl(options.url);
        if (options.auth !== undefined) {
            throw mysqlConfigurationError('address_conflict', 'auth must not be set together with url: provide credentials in exactly one of them');
        }
        if (options.database !== undefined && parsed.database !== undefined) {
            throw mysqlConfigurationError('address_conflict', 'database must not be set together with a url path: provide it in exactly one of them');
        }
        return {
            endpoint: parsed.endpoint,
            database: options.database ?? parsed.database,
            auth: options.auth ?? parsed.auth,
            charset: options.charset,
            connectTimeoutMs: options.connectTimeoutMs,
        };
    }

    const endpoint =
        options.endpoint === undefined
            ? { host: '127.0.0.1', port: 3306 }
            : validateEndpoint(options.endpoint, 'endpoint');
    return {
        endpoint,
        database: options.database,
        auth: options.auth,
        charset: options.charset,
        connectTimeoutMs: options.connectTimeoutMs,
    };
}

function parseMySQLUrl(url: string): { endpoint: TcpEndpoint; database?: string; auth?: PasswordAuth } {
    let parsed: URL;
    try {
        parsed = new URL(url);
    } catch (error) {
        throw mysqlConfigurationError('invalid_url', `url is not a valid mysql:// URL: ${url}`, error);
    }
    if (parsed.protocol !== 'mysql:' && parsed.protocol !== 'mysql+tcp:') {
        throw mysqlConfigurationError('invalid_url', `url scheme must be mysql:// or mysql+tcp://, received ${parsed.protocol}`);
    }
    if (parsed.search !== '') {
        throw mysqlConfigurationError('invalid_url', 'url query parameters are not supported yet; pass settings as structured options');
    }
    const host = parsed.hostname === '' ? '127.0.0.1' : decodeURIComponent(parsed.hostname);
    const port = parsed.port === '' ? 3306 : Number(parsed.port);
    const endpoint = validateEndpoint({ host, port }, 'url');
    const database = decodeURIComponent(parsed.pathname.replace(/^\//, ''));
    const auth =
        parsed.username === '' && parsed.password === ''
            ? undefined
            : { username: decodeURIComponent(parsed.username), password: decodeURIComponent(parsed.password) };
    return {
        endpoint,
        ...(database === '' ? {} : { database }),
        ...(auth === undefined ? {} : { auth }),
    };
}

/** Isolation levels accepted by `transaction`, mapped to MySQL's legal set. */
export const ISOLATION_LEVELS: readonly MySQLIsolationLevel[] = ['read-uncommitted', 'read-committed', 'repeatable-read', 'serializable'];

/** Shape check so obvious call mistakes fail before touching the native layer. */
export function validateSql(sql: unknown): void {
    if (typeof sql !== 'string' || sql.trim() === '') {
        throw mysqlConfigurationError('invalid_sql', 'sql must be a non-empty string');
    }
}

/**
 * Shape check for `?` parameters. Values are handed to the native prepared
 * protocol untouched; nothing here escapes or concatenates them into SQL.
 */
export function validateParams(params: readonly MySQLValue[] | undefined): void {
    if (params === undefined) return;
    if (!Array.isArray(params)) {
        throw mysqlConfigurationError('invalid_params', 'params must be an array of null, number, bigint, string or Uint8Array');
    }
    for (const value of params) {
        if (
            value === null ||
            typeof value === 'number' ||
            typeof value === 'bigint' ||
            typeof value === 'string' ||
            value instanceof Uint8Array
        ) {
            continue;
        }
        throw mysqlConfigurationError(
            'invalid_params',
            `unsupported parameter value of type ${typeof value}: parameters bind through the prepared protocol only`,
        );
    }
}

/** Shape check for per-request options. */
export function validateRequestOptions(options: RequestOptions | undefined): void {
    if (options?.timeoutMs !== undefined && (!Number.isInteger(options.timeoutMs) || options.timeoutMs < 0)) {
        throw mysqlConfigurationError('invalid_request_options', 'timeoutMs must be an integer number of milliseconds at least 0');
    }
    if (options?.signal !== undefined && !(options.signal instanceof AbortSignal)) {
        throw mysqlConfigurationError('invalid_request_options', 'signal must be an AbortSignal');
    }
}

/** Shape check for `withSession` / `transaction` callbacks. */
export function validateCallback(fn: unknown, call: string): void {
    if (typeof fn !== 'function') {
        throw mysqlConfigurationError('invalid_callback', `${call} requires a function`);
    }
}

/** Shape check for transaction options against MySQL's legal set. */
export function validateTransactionOptions(options: { isolation?: MySQLIsolationLevel; readOnly?: boolean } | undefined): void {
    if (options?.isolation !== undefined && !ISOLATION_LEVELS.includes(options.isolation)) {
        throw mysqlConfigurationError(
            'invalid_isolation',
            `isolation must be one of ${ISOLATION_LEVELS.join(', ')}, received ${JSON.stringify(options.isolation)}`,
        );
    }
    if (options?.readOnly !== undefined && typeof options.readOnly !== 'boolean') {
        throw mysqlConfigurationError('invalid_transaction_options', 'readOnly must be a boolean');
    }
}
