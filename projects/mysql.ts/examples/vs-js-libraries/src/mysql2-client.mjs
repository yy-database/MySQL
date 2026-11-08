import mysql from "mysql2/promise";

// Thin wrapper so both benchmark paths issue the same SQL and expose the
// same shape: `query(sql)` returns `{ columns, rows }` with raw driver rows.
export async function connectMysql2(port, host = "127.0.0.1") {
  const connection = await mysql.createConnection({
    host,
    port,
    user: "yydb",
    password: "",
    // One text-protocol connection, no pooling, no prepared statements, no TLS.
    disableEval: true,
  });

  return {
    async query(sql) {
      const [rows, fields] = await connection.query(sql);
      const columns = Array.isArray(fields) ? fields.map((field) => field.name) : [];
      return { columns, rows };
    },
    async close() {
      await connection.end();
    },
  };
}
