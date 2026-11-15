import mysql from "mysql2/promise";

// Thin wrapper so both benchmark paths issue the same SQL and expose the
// same shape: `query(sql)` returns `{ columns, rows }` with raw driver rows and
// `prepare(sql)` mirrors the minimal client's binary-protocol handle.
export async function connectMysql2(port, host = "127.0.0.1") {
  const connection = await mysql.createConnection({
    host,
    port,
    user: "yydb",
    password: "",
    // One text-protocol connection, no pooling, no TLS.
    disableEval: true,
  });

  return {
    async query(sql) {
      const [rows, fields] = await connection.query(sql);
      const columns = Array.isArray(fields) ? fields.map((field) => field.name) : [];
      // DML answers with a ResultSetHeader instead of a row array; surface its
      // fields so benchmarks can assert generated ids the same way as the
      // minimal client.
      const header = !Array.isArray(rows) && rows !== null && typeof rows === "object" ? rows : undefined;
      return {
        columns,
        rows,
        affectedRows: header?.affectedRows ?? (Array.isArray(rows) ? rows.length : 0),
        insertId: header?.insertId ?? 0,
      };
    },
    async prepare(sql) {
      const statement = await connection.prepare(sql);
      return {
        async execute(parameters = []) {
          const [rows, fields] = await statement.execute(parameters);
          const columns = Array.isArray(fields) ? fields.map((field) => field.name) : [];
          return { columns, rows, affectedRows: Array.isArray(rows) ? rows.length : rows.affectedRows };
        },
        async close() {
          await statement.close();
        },
      };
    },
    async close() {
      await connection.end();
    },
  };
}
