// The MySQL product error envelope. Every failure raised through a public
// constructor carries the same locator fields, so callers branch on `phase`,
// `execution` and `retry` instead of parsing messages. Server-originated
// failures additionally carry `errno` and `sqlState` from the wire ERR frame;
// locally raised failures leave both undefined rather than inventing codes.

/** Lifecycle stage that produced the failure. */
export type MySQLErrorPhase = 'configuration' | 'open' | 'connect' | 'execute' | 'commit' | 'close';

/** Whether the underlying request reached the executor or the remote server. */
export type MySQLErrorExecution = 'not-sent' | 'unknown' | 'replied';

/** Whether repeating the operation is safe. Advisory only: nothing retries automatically. */
export type MySQLErrorRetry = 'never' | 'safe' | 'application-decision';

/** Fields accepted by {@link MySQLDatabaseError}. */
export interface MySQLErrorInit {
    readonly code: string;
    readonly message: string;
    readonly phase: MySQLErrorPhase;
    readonly execution: MySQLErrorExecution;
    readonly retry: MySQLErrorRetry;
    readonly errno?: number;
    readonly sqlState?: string;
    readonly cause?: unknown;
}

/** A MySQL failure with the family error envelope attached. */
export class MySQLDatabaseError extends Error {
    /** Product identity of the envelope. */
    readonly product = 'mysql' as const;
    /** Stable machine-readable reason. */
    readonly code: string;
    /** Server ERR packet errno, when the failure came from a server reply. */
    readonly errno: number | undefined;
    /** Server ERR packet SQLSTATE, when the failure came from a server reply. */
    readonly sqlState: string | undefined;
    /** Lifecycle stage that produced the failure. */
    readonly phase: MySQLErrorPhase;
    /** Whether the request was sent and how it ended. */
    readonly execution: MySQLErrorExecution;
    /** Whether repeating the operation is safe. */
    readonly retry: MySQLErrorRetry;

    constructor(init: MySQLErrorInit) {
        super(init.message, { cause: init.cause });
        this.name = 'MySQLDatabaseError';
        this.code = init.code;
        this.errno = init.errno;
        this.sqlState = init.sqlState;
        this.phase = init.phase;
        this.execution = init.execution;
        this.retry = init.retry;
    }
}

/** Failure while validating constructor or call options. */
export function mysqlConfigurationError(code: string, message: string, cause?: unknown): MySQLDatabaseError {
    return new MySQLDatabaseError({ code, message, phase: 'configuration', execution: 'not-sent', retry: 'never', cause });
}

/** Failure because the object's lifecycle state does not allow the call. */
export function mysqlStateError(code: string, message: string, phase: MySQLErrorPhase, cause?: unknown): MySQLDatabaseError {
    return new MySQLDatabaseError({ code, message, phase, execution: 'not-sent', retry: 'never', cause });
}

/** Failure surfaced by a native primitive. */
export function mysqlNativeError(
    code: string,
    message: string,
    cause: unknown,
    phase: MySQLErrorPhase,
    execution: MySQLErrorExecution = 'not-sent',
    retry: MySQLErrorRetry = 'application-decision',
): MySQLDatabaseError {
    return new MySQLDatabaseError({ code, message, phase, execution, retry, cause });
}

/**
 * Failure because the exported native primitives do not (yet) provide the
 * capability. The request is never sent and no substitute runs in TypeScript:
 * a second SQL executor, protocol codec or pool in this package is forbidden.
 */
export function mysqlCapabilityGapError(capability: string, phase: MySQLErrorPhase): MySQLDatabaseError {
    return new MySQLDatabaseError({
        code: 'native_capability_missing',
        message: `the native binding does not export ${capability}; this call is not implemented in TypeScript on purpose, install or upgrade a platform binding that provides it`,
        phase,
        execution: 'not-sent',
        retry: 'never',
    });
}

/** Text of an unknown thrown value, for wrapping into the envelope. */
export function errorText(error: unknown): string {
    if (error instanceof Error) return error.message;
    return String(error);
}
