// Author-facing result, value and session contracts. These types describe the
// shape the native primitives return once exported; this file never decodes
// wire bytes or guesses types — value restoration belongs to the FFI layer and
// the upstream codec, driven by real column metadata.

import type { RequestOptions } from './options.ts';

/**
 * A bindable parameter. `bigint` covers `BIGINT`/unsigned `BIGINT`,
 * `Uint8Array` carries raw octets bound through the prepared binary protocol,
 * and exact decimals travel as strings until the native binding provides a
 * precise wrapper type.
 */
export type MySQLValue = null | number | bigint | string | Uint8Array;

/** One result row keyed by column name. Duplicate names require array rows. */
export type MySQLRow = Readonly<Record<string, MySQLValue>>;

/** Column metadata as reported by the server. */
export interface MySQLColumn {
    /** Column label (`SELECT … AS name` becomes `name`). */
    readonly name: string;
    /** `MYSQL_TYPE_*` code from the column definition. */
    readonly typeCode: number;
    /** Whether the column is declared unsigned. */
    readonly unsigned: boolean;
    /** Whether the column carries raw octets rather than text. */
    readonly binary: boolean;
}

/** One diagnostic attached to a write result. */
export interface MySQLWarning {
    /** Server warning code. */
    readonly code: number;
    /** Server warning level, for example `Note` or `Warning`. */
    readonly level: string;
    /** Human-readable warning text. */
    readonly message: string;
}

/** Rows of one statement that returns a result set. */
export interface MySQLResult<R = MySQLRow> {
    readonly rows: readonly R[];
    readonly columns: readonly MySQLColumn[];
}

/** Outcome of one statement that writes. */
export interface MySQLWriteResult {
    readonly affectedRows: bigint;
    readonly lastInsertId: bigint | null;
    readonly warnings: readonly MySQLWarning[];
}

/** Options for `query`. */
export interface MySQLQueryOptions extends RequestOptions {
    /** Return `MySQLValue[]` rows positionally instead of name-keyed objects. */
    readonly arrayRows?: boolean;
}

/** Isolation levels accepted by `transaction`, mapped to MySQL's legal set. */
export type MySQLIsolationLevel = 'read-uncommitted' | 'read-committed' | 'repeatable-read' | 'serializable';

/** Options for `transaction`. */
export interface MySQLTransactionOptions {
    readonly isolation?: MySQLIsolationLevel;
    readonly readOnly?: boolean;
}

/** Statements available on a borrowed session or an active transaction. */
export interface MySQLOperations {
    /** Run one statement that returns a row set. */
    query<R = MySQLRow>(sql: string, params?: readonly MySQLValue[], options?: MySQLQueryOptions): Promise<MySQLResult<R>>;
    /** Run one statement that writes. */
    execute(sql: string, params?: readonly MySQLValue[], options?: RequestOptions): Promise<MySQLWriteResult>;
}

/**
 * One physical session borrowed through `withSession`. Session variables,
 * temporary tables and prepared statements belong to it and stop being
 * reachable when the scope ends.
 */
export interface MySQLSession extends MySQLOperations {}

/**
 * One transaction fixed to a single session. The callback runs at most once;
 * begin, commit and rollback are decided by the native state machine, never
 * by string commands composed in TypeScript.
 */
export interface MySQLTransaction extends MySQLOperations {}
