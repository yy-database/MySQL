import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { startMysqlServer } from "../../../packages/mysql/dist/index.js";
import { connectMysql } from "../src/mysql-client.mjs";
import { connectMysql2 } from "../src/mysql2-client.mjs";

const ROWS_SQL = "SELECT id, label, score FROM conformance_items ORDER BY id";

function asCells(result) {
  if (Array.isArray(result.rows[0])) return result.rows;
  return result.rows.map((row) => result.columns.map((column) => row[column]));
}

function normalize(cells) {
  return cells.map((row) => row.map((cell) => (cell === null ? "<null>" : String(cell))));
}

async function main() {
  const dataDir = await mkdtemp(join(tmpdir(), "mysql-conformance-"));
  const server = startMysqlServer({ dataDir, port: 0 });
  const mysql2 = await connectMysql2(server.port);
  const client = await connectMysql(server.port);

  try {
    await mysql2.query(
      "CREATE TABLE conformance_items (id INT PRIMARY KEY, label VARCHAR(40), score DOUBLE)",
    );
    for (const [id, label, score] of [
      [3, "three", 7.75],
      [1, "one", 0.25],
      [2, "two", 2.5],
    ]) {
      await mysql2.query(
        `INSERT INTO conformance_items (id, label, score) VALUES (${id}, '${label}', ${score})`,
      );
    }

    const viaMinimal = normalize(asCells(await client.query(ROWS_SQL)));
    const expected = [
      ["1", "one", "0.25"],
      ["2", "two", "2.5"],
      ["3", "three", "7.75"],
    ];
    assert.deepEqual(viaMinimal, expected);
    console.log("PASS mysql2 write -> minimal client read (exact cells, ordered)");

    await client.query("INSERT INTO conformance_items (id, label, score) VALUES (4, 'four', 9.5)");
    const inserted = normalize(asCells(await mysql2.query(ROWS_SQL)));
    assert.equal(inserted.length, 4);
    assert.deepEqual(inserted[3], ["4", "four", "9.5"]);
    console.log("PASS minimal client write -> mysql2 read");

    await client.query("UPDATE conformance_items SET label = 'FOUR', score = 10.5 WHERE id = 4");
    const updated = normalize(asCells(await mysql2.query("SELECT id, label, score FROM conformance_items WHERE id = 4")));
    assert.deepEqual(updated, [["4", "FOUR", "10.5"]]);
    console.log("PASS minimal client update -> mysql2 read");

    await client.query("DELETE FROM conformance_items WHERE id = 4");
    const deleted = await mysql2.query("SELECT id FROM conformance_items WHERE id = 4");
    assert.equal(deleted.rows.length, 0);
    console.log("PASS minimal client delete -> mysql2 read");

    for (const [id, label, score] of [
      [10, "ten", 1.5],
      [7, "seven", 4.5],
    ]) {
      await mysql2.query(
        `INSERT INTO conformance_items (id, label, score) VALUES (${id}, '${label}', ${score})`,
      );
    }
    const scanMinimal = normalize(asCells(await client.query(ROWS_SQL)));
    const scanMysql2 = normalize(asCells(await mysql2.query(ROWS_SQL)));
    assert.deepEqual(scanMinimal, scanMysql2);
    const ids = scanMinimal.map((row) => Number(row[0]));
    assert.deepEqual(ids, [1, 2, 3, 7, 10]);
    console.log("PASS ORDER BY id scan matches on both clients");

    console.log("CONFORMANCE PASS");
  } finally {
    await client.close();
    await mysql2.close();
    server.stop();
    await rm(dataDir, { recursive: true, force: true }).catch(() => {});
  }
}

try {
  await main();
} catch (error) {
  console.error("CONFORMANCE FAIL");
  console.error(error);
  process.exitCode = 1;
}
