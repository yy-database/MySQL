// Side-by-side throughput with `mysql2` as baseline.
//
// Fairness rules:
// - every (round × workload × client) starts a fresh `startMysqlServer` with a
//   fresh `mkdtemp` data directory;
// - table creation and seeding are untimed;
// - both clients use one connection, no pooling, no TLS;
// - text-protocol workloads share one SQL and one seed size across clients;
// - `insert-prepared` is the exception: both clients prepare once and execute
//   the same binary-protocol statement `BENCH_ROWS` times;
// - `connect-handshake` opens and closes its own extra connections to the same
//   fresh server while the measurement connection stays open;
// - the server and client are stopped after each measurement.
//
// Run: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries benchmark`
// Env: BENCH_ROWS (200), BENCH_POINT (500), BENCH_SCAN (1000), BENCH_ROUNDS (3),
//      BENCH_TX (10), BENCH_TX_ROWS (10), BENCH_MUTATE (100), BENCH_ERROR (100),
//      BENCH_PAGE (20), BENCH_JOIN (100), BENCH_SHOW (200), BENCH_BATCH_ROWS (20),
//      BENCH_AGG (100), BENCH_CONNECT (20), BENCH_ALTER (10),
//      BENCH_ONLY (comma-separated names)

import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { startMysqlServer } from "../../../packages/mysql/dist/index.js";
import { connectMysql } from "../src/mysql-client.mjs";
import { connectMysql2 } from "../src/mysql2-client.mjs";
import { elapsedMs, median, p95, throughput } from "../src/timing.mjs";

const ROWS = Number(process.env.BENCH_ROWS ?? 200);
const POINT_OPS = Number(process.env.BENCH_POINT ?? 500);
const SCAN_ROWS = Number(process.env.BENCH_SCAN ?? 1_000);
const ROUNDS = Number(process.env.BENCH_ROUNDS ?? 3);
const TX_COUNT = Number(process.env.BENCH_TX ?? 10);
const TX_ROWS = Number(process.env.BENCH_TX_ROWS ?? 10);
const MUTATE_OPS = Number(process.env.BENCH_MUTATE ?? 100);
const ERROR_OPS = Number(process.env.BENCH_ERROR ?? 100);
const PAGE_OPS = Number(process.env.BENCH_PAGE ?? 20);
const JOIN_OPS = Number(process.env.BENCH_JOIN ?? 100);
const SHOW_OPS = Number(process.env.BENCH_SHOW ?? 200);
const BATCH_ROWS = Number(process.env.BENCH_BATCH_ROWS ?? 20);
const AGG_OPS = Number(process.env.BENCH_AGG ?? 100);
const CONNECT_OPS = Number(process.env.BENCH_CONNECT ?? 20);
const CURSOR_PAGE_SIZE = 20;
const RANGE_ROWS = 20;
const WINDOW_OPS = Number(process.env.BENCH_WINDOW ?? 20);
const SAVEPOINT_OPS = Number(process.env.BENCH_SAVEPOINT ?? 20);
const ALTER_OPS = Number(process.env.BENCH_ALTER ?? 10);
const GROUP_SLOTS = 8;
// Fixed predicate shapes so every client issues identical SQL.
const LIKE_PREFIX_ROWS = (() => {
  let matched = 0;
  for (let id = 1; id <= ROWS; id += 1) if (String(id).startsWith("1")) matched += 1;
  return matched;
})();
const IN_LIST_VALUES = Array.from({ length: 17 }, (_, index) => index * 11 + 3).filter((id) => id <= ROWS);
const IN_LIST_IDS = IN_LIST_VALUES.join(", ");
const IN_LIST_ROWS = IN_LIST_VALUES.length;
const SUBQUERY_BOUND = Math.floor(ROWS * 0.6);
// Left-join seed: side rows only exist for odd item ids, so the join must pad
// exactly the even ones with NULL.
const SIDE_HIT_ROWS = Math.ceil(ROWS / 2);
const SIDE_NULL_ROWS = ROWS - SIDE_HIT_ROWS;
// `HAVING` seed: `grp = id % 8`, `val = id`. The threshold is the median group
// sum, so roughly half the groups survive the predicate for any `BENCH_ROWS`.
const GROUP_COUNTS = new Array(GROUP_SLOTS).fill(0);
const GROUP_SUMS = new Array(GROUP_SLOTS).fill(0);
for (let id = 1; id <= ROWS; id += 1) {
  const slot = id % GROUP_SLOTS;
  GROUP_COUNTS[slot] += 1;
  GROUP_SUMS[slot] += id;
}
const HAVING_SUM_MIN = (() => {
  const sums = GROUP_SUMS.filter((sum) => sum > 0).sort((total, other) => total - other);
  return sums.length === 0 ? 0 : sums[Math.floor(sums.length / 2)];
})();
const HAVING_GROUPS = GROUP_SUMS.map((sum, slot) => ({ slot, sum, count: GROUP_COUNTS[slot] })).filter(
  (group) => group.sum > HAVING_SUM_MIN,
);
// `CASE` thresholds cycle 1..4 over `score = id / 4`, i.e. the `hi` band is
// `id > 4 * threshold`.
const CASE_THRESHOLD = (index) => (index % 4) + 1;
// Narrow the run to a subset of workloads, e.g. `BENCH_ONLY=select-point-uk,index-range`.
const ONLY = (process.env.BENCH_ONLY ?? "")
  .split(",")
  .map((name) => name.trim())
  .filter(Boolean);

const CLIENTS = [
  { name: "mysql2", connect: connectMysql2 },
  { name: "@yydb/mysql", connect: connectMysql },
];

const CREATE_TABLE = "CREATE TABLE bench_items (id INT PRIMARY KEY, label VARCHAR(40), score DOUBLE)";
const CREATE_TABLE_UK =
  "CREATE TABLE bench_items_uk (id INT PRIMARY KEY, uk INT UNIQUE, label VARCHAR(40), score DOUBLE)";
const CREATE_TABLE_LARGE = "CREATE TABLE bench_large (id INT PRIMARY KEY, note VARCHAR(40) NULL, payload TEXT)";
const CREATE_TABLE_AUTO = "CREATE TABLE bench_auto (id INT PRIMARY KEY AUTO_INCREMENT, label VARCHAR(40), score DOUBLE)";
const CREATE_TABLE_SIDE = "CREATE TABLE bench_side (id INT PRIMARY KEY, item_id INT, weight DOUBLE)";
const CREATE_TABLE_SPARSE_SIDE =
  "CREATE TABLE bench_side_sparse (id INT PRIMARY KEY, item_id INT, weight DOUBLE)";
const CREATE_TABLE_GROUPS = "CREATE TABLE bench_grp (id INT PRIMARY KEY, grp INT, val INT)";

async function seed(client, rows) {
  await client.query(CREATE_TABLE);
  for (let id = 1; id <= rows; id += 1) {
    await client.query(
      `INSERT INTO bench_items (id, label, score) VALUES (${id}, 'row-${id}', ${id / 4})`,
    );
  }
}

async function seedUnique(client, rows) {
  await client.query(CREATE_TABLE_UK);
  for (let id = 1; id <= rows; id += 1) {
    await client.query(
      `INSERT INTO bench_items_uk (id, uk, label, score) VALUES (${id}, ${id}, 'row-${id}', ${id / 4})`,
    );
  }
}

// Large/NULL mix: every third row has a NULL note, every fifth row carries a
// 2 KiB payload. Batched seed keeps untimed setup bounded at 1,000 rows.
async function seedLarge(client, rows) {
  await client.query(CREATE_TABLE_LARGE);
  const batch = 100;
  for (let start = 1; start <= rows; start += batch) {
    const values = [];
    for (let id = start; id < Math.min(start + batch, rows + 1); id += 1) {
      const note = id % 3 === 0 ? "NULL" : `'note-${id}'`;
      const payload = id % 5 === 0 ? "x".repeat(2048) : "small";
      values.push(`(${id}, ${note}, '${payload}')`);
    }
    await client.query(`INSERT INTO bench_large (id, note, payload) VALUES ${values.join(", ")}`);
  }
}

// One side row per item id so the join below has a fixed result cardinality.
async function seedSide(client, rows) {
  await client.query(CREATE_TABLE_SIDE);
  const batch = 100;
  for (let start = 1; start <= rows; start += batch) {
    const values = [];
    for (let id = start; id < Math.min(start + batch, rows + 1); id += 1) {
      values.push(`(${id}, ${id}, ${id / 4})`);
    }
    await client.query(`INSERT INTO bench_side (id, item_id, weight) VALUES ${values.join(", ")}`);
  }
}

// Only odd item ids get a side row, so `LEFT JOIN` has a fixed NULL count.
async function seedSparseSide(client, rows) {
  await client.query(CREATE_TABLE_SPARSE_SIDE);
  const batch = 100;
  for (let start = 1; start <= rows; start += batch) {
    const values = [];
    for (let id = start; id < Math.min(start + batch, rows + 1); id += 2) {
      values.push(`(${id}, ${id}, ${id / 4})`);
    }
    if (values.length > 0) {
      await client.query(`INSERT INTO bench_side_sparse (id, item_id, weight) VALUES ${values.join(", ")}`);
    }
  }
}

// `grp = id % 8` and `val = id`, so `HAVING SUM(val) > median(group sums)` is
// selective without depending on how rows happen to be ordered.
async function seedGroups(client, rows) {
  await client.query(CREATE_TABLE_GROUPS);
  const batch = 100;
  for (let start = 1; start <= rows; start += batch) {
    const values = [];
    for (let id = start; id < Math.min(start + batch, rows + 1); id += 1) {
      values.push(`(${id}, ${id % GROUP_SLOTS}, ${id})`);
    }
    await client.query(`INSERT INTO bench_grp (id, grp, val) VALUES ${values.join(", ")}`);
  }
}

function cellAt(result, row, index) {
  return Array.isArray(row) ? row[index] : row[result.columns[index]];
}

// Both clients surface the server error number as `errno`; a successful call
// means the gateway stopped rejecting the statement, so the run must fail.
async function expectError(client, sql, errno) {
  try {
    await client.query(sql);
  } catch (error) {
    if (error.errno !== errno) {
      throw new Error(`expected errno ${errno}, got ${error.errno}: ${error.message}`, { cause: error });
    }
    return;
  }
  throw new Error(`expected errno ${errno} but the statement succeeded: ${sql}`);
}

const ALL_WORKLOADS = [
  {
    name: "connect-handshake",
    unit: "connections",
    ops: CONNECT_OPS,
    async setup() {},
    async run(_client, open) {
      // Handshake v10 plus auth round trip against the same fresh server; the
      // measurement connection stays open and every iteration adds one more.
      for (let index = 0; index < CONNECT_OPS; index += 1) {
        const connection = await open();
        try {
          const result = await connection.query("SELECT 1");
          if (result.rows.length !== 1) {
            throw new Error(`handshake probe rows=${result.rows.length}`);
          }
        } finally {
          await connection.close();
        }
      }
      return CONNECT_OPS;
    },
  },
  {
    name: "insert",
    unit: "statements",
    ops: ROWS,
    async setup(client) {
      await client.query(CREATE_TABLE);
    },
    async run(client) {
      for (let id = 1; id <= ROWS; id += 1) {
        await client.query(
          `INSERT INTO bench_items (id, label, score) VALUES (${id}, 'row-${id}', ${id / 4})`,
        );
      }
      return ROWS;
    },
  },
  {
    name: "insert-prepared",
    unit: "rows",
    ops: ROWS,
    async setup(client) {
      await client.query(CREATE_TABLE);
    },
    async run(client) {
      // Same SQL and row count as `insert`, but one prepare followed by
      // `ROWS` binary-protocol executes.
      const statement = await client.prepare(
        "INSERT INTO bench_items (id, label, score) VALUES (?, ?, ?)",
      );
      try {
        for (let id = 1; id <= ROWS; id += 1) {
          await statement.execute([id, `row-${id}`, id / 4]);
        }
      } finally {
        await statement.close();
      }
      return ROWS;
    },
  },
  {
    name: "insert-batch",
    unit: "rows",
    ops: ROWS,
    async setup(client) {
      await client.query(CREATE_TABLE);
    },
    async run(client) {
      // Same row count as `insert`, but one multi-row VALUES statement per
      // `BATCH_ROWS` rows, which is how bulk loaders actually write.
      for (let start = 1; start <= ROWS; start += BATCH_ROWS) {
        const values = [];
        for (let id = start; id < Math.min(start + BATCH_ROWS, ROWS + 1); id += 1) {
          values.push(`(${id}, 'row-${id}', ${id / 4})`);
        }
        await client.query(`INSERT INTO bench_items (id, label, score) VALUES ${values.join(", ")}`);
      }
      return ROWS;
    },
  },
  {
    name: "insert-autoinc",
    unit: "rows",
    ops: ROWS,
    async setup(client) {
      await client.query(CREATE_TABLE_AUTO);
    },
    async run(client) {
      // Same row count as `insert`, but the primary key is omitted so the
      // server allocates it and reports the value in the OK packet.
      for (let id = 1; id <= ROWS; id += 1) {
        const result = await client.query(
          `INSERT INTO bench_auto (label, score) VALUES ('row-${id}', ${id / 4})`,
        );
        if (Number(result.insertId) !== id) {
          throw new Error(`autoinc insertId=${result.insertId}, expected=${id}`);
        }
      }
      return ROWS;
    },
  },
  {
    name: "select-point",
    unit: "statements",
    ops: POINT_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      for (let index = 0; index < POINT_OPS; index += 1) {
        const id = (index % ROWS) + 1;
        const result = await client.query(`SELECT label FROM bench_items WHERE id = ${id}`);
        if (result.rows.length !== 1) throw new Error(`point miss id=${id}`);
      }
      return POINT_OPS;
    },
  },
  {
    name: "select-point-uk",
    unit: "statements",
    ops: POINT_OPS,
    async setup(client) {
      await seedUnique(client, ROWS);
    },
    async run(client) {
      // Same cardinality as `select-point`; only the predicate moves from the
      // primary key to a UNIQUE secondary index.
      for (let index = 0; index < POINT_OPS; index += 1) {
        const uk = (index % ROWS) + 1;
        const result = await client.query(
          `SELECT label FROM bench_items_uk WHERE uk = ${uk}`,
        );
        if (result.rows.length !== 1) throw new Error(`point-uk miss uk=${uk}`);
      }
      return POINT_OPS;
    },
  },
  {
    name: "index-range",
    unit: "statements",
    ops: POINT_OPS,
    async setup(client) {
      await seedUnique(client, ROWS);
    },
    async run(client) {
      // `RANGE_ROWS` of the seeded `uk` values per statement, the shape a
      // secondary-index range scan serves. `ORDER BY id` keeps both clients on
      // identical SQL and fails if the index scan reorders rows.
      const span = Math.max(1, ROWS - RANGE_ROWS);
      for (let index = 0; index < POINT_OPS; index += 1) {
        const from = (index % span) + 1;
        const result = await client.query(
          `SELECT id FROM bench_items_uk WHERE uk BETWEEN ${from} AND ${from + RANGE_ROWS - 1} ORDER BY id`,
        );
        if (result.rows.length !== RANGE_ROWS) {
          throw new Error(`index-range from=${from} rows=${result.rows.length}`);
        }
        const first = Number(cellAt(result, result.rows[0], 0));
        if (first !== from) {
          throw new Error(`index-range from=${from} first=${first}`);
        }
      }
      return POINT_OPS;
    },
  },
  {
    name: "select-range",
    unit: "statements",
    ops: POINT_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      // Primary-key range of `RANGE_ROWS` rows, the shape a range index scan
      // serves. The strict lower bound matches the MySQL 8.4 comparison script,
      // which found inclusive-lower predicates dropping boundary rows here.
      const span = Math.max(1, ROWS - RANGE_ROWS);
      for (let index = 0; index < POINT_OPS; index += 1) {
        const from = (index % span) + 1;
        const result = await client.query(
          `SELECT id, label FROM bench_items WHERE id > ${from} AND id <= ${from + RANGE_ROWS} ORDER BY id`,
        );
        if (result.rows.length !== RANGE_ROWS) {
          throw new Error(`select-range from=${from} rows=${result.rows.length}`);
        }
        const first = Number(cellAt(result, result.rows[0], 0));
        if (first !== from + 1) {
          throw new Error(`select-range from=${from} first=${first}`);
        }
      }
      return POINT_OPS;
    },
  },
  {
    name: "select-scan",
    unit: "rows",
    ops: SCAN_ROWS,
    async setup(client) {
      await seed(client, SCAN_ROWS);
    },
    async run(client) {
      const result = await client.query("SELECT id, label, score FROM bench_items ORDER BY id");
      if (result.rows.length !== SCAN_ROWS) {
        throw new Error(`${result.rows.length} rows, expected ${SCAN_ROWS}`);
      }
      return SCAN_ROWS;
    },
  },
  {
    name: "aggregate",
    unit: "statements",
    ops: AGG_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      // One-row aggregate over the same seed; COUNT/SUM/MIN/MAX cover the
      // aggregate pipeline without an ORDER BY row payload.
      for (let index = 0; index < AGG_OPS; index += 1) {
        const result = await client.query(
          "SELECT COUNT(*) AS n, SUM(score) AS s, MIN(score) AS lo, MAX(score) AS hi FROM bench_items",
        );
        if (result.rows.length !== 1) {
          throw new Error(`aggregate rows=${result.rows.length}`);
        }
        const count = Number(cellAt(result, result.rows[0], 0));
        if (count !== ROWS) {
          throw new Error(`aggregate count=${count}, expected=${ROWS}`);
        }
      }
      return AGG_OPS;
    },
  },
  {
    name: "select-group-by",
    unit: "statements",
    ops: AGG_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      // `label` is unique, so this is the worst case for the grouping map:
      // `ROWS` groups over `ROWS` rows and nothing collapses.
      for (let index = 0; index < AGG_OPS; index += 1) {
        const result = await client.query("SELECT label, COUNT(*) AS n FROM bench_items GROUP BY label");
        if (result.rows.length !== ROWS) {
          throw new Error(`group-by groups=${result.rows.length}, expected ${ROWS}`);
        }
        let total = 0;
        for (const row of result.rows) total += Number(cellAt(result, row, 1));
        if (total !== ROWS) throw new Error(`group-by total=${total}, expected ${ROWS}`);
      }
      return AGG_OPS;
    },
  },
  {
    name: "select-having",
    unit: "statements",
    ops: AGG_OPS,
    async setup(client) {
      await seedGroups(client, ROWS);
    },
    async run(client) {
      // `HAVING SUM(val)` keeps the same 8-slot grouping as `select-group-by`
      // but filters after aggregation; the threshold is fixed in SQL, so both
      // clients issue identical statements.
      for (let index = 0; index < AGG_OPS; index += 1) {
        const result = await client.query(
          `SELECT grp, COUNT(*) AS n FROM bench_grp GROUP BY grp HAVING SUM(val) > ${HAVING_SUM_MIN} ORDER BY grp`,
        );
        if (result.rows.length !== HAVING_GROUPS.length) {
          throw new Error(`having groups=${result.rows.length}, expected=${HAVING_GROUPS.length}`);
        }
        for (let row = 0; row < result.rows.length; row += 1) {
          const group = Number(cellAt(result, result.rows[row], 0));
          const count = Number(cellAt(result, result.rows[row], 1));
          if (group !== HAVING_GROUPS[row].slot) {
            throw new Error(`having grp=${group}, expected=${HAVING_GROUPS[row].slot}`);
          }
          if (count !== HAVING_GROUPS[row].count) {
            throw new Error(`having grp=${group} n=${count}, expected=${HAVING_GROUPS[row].count}`);
          }
        }
      }
      return AGG_OPS;
    },
  },
  {
    name: "select-case",
    unit: "statements",
    ops: AGG_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      // `CASE` in the select list over the same seed; the threshold cycles 1..4
      // so the `hi` band size differs per statement while both clients still
      // issue byte-identical SQL.
      for (let index = 0; index < AGG_OPS; index += 1) {
        const threshold = CASE_THRESHOLD(index);
        const boundary = threshold * 4;
        const expectedHigh = Math.max(0, ROWS - boundary);
        const result = await client.query(
          `SELECT id, CASE WHEN score > ${threshold} THEN 'hi' ELSE 'lo' END AS band FROM bench_items ORDER BY id`,
        );
        if (result.rows.length !== ROWS) {
          throw new Error(`case rows=${result.rows.length}, expected=${ROWS}`);
        }
        let high = 0;
        for (const row of result.rows) {
          const id = Number(cellAt(result, row, 0));
          const band = String(cellAt(result, row, 1));
          if (band !== (id > boundary ? "hi" : "lo")) {
            throw new Error(`case id=${id} band=${band} threshold=${threshold}`);
          }
          if (band === "hi") high += 1;
        }
        if (high !== expectedHigh) {
          throw new Error(`case threshold=${threshold} high=${high}, expected=${expectedHigh}`);
        }
      }
      return AGG_OPS;
    },
  },
  {
    name: "select-distinct",
    unit: "statements",
    ops: AGG_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      for (let index = 0; index < AGG_OPS; index += 1) {
        const result = await client.query("SELECT DISTINCT label FROM bench_items");
        if (result.rows.length !== ROWS) {
          throw new Error(`distinct rows=${result.rows.length}, expected ${ROWS}`);
        }
      }
      return AGG_OPS;
    },
  },
  {
    name: "select-like-prefix",
    unit: "statements",
    ops: AGG_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      for (let index = 0; index < AGG_OPS; index += 1) {
        const result = await client.query("SELECT COUNT(*) AS n FROM bench_items WHERE label LIKE 'row-1%'");
        const matched = Number(cellAt(result, result.rows[0], 0));
        if (matched !== LIKE_PREFIX_ROWS) throw new Error(`like-prefix matched=${matched}, expected=${LIKE_PREFIX_ROWS}`);
      }
      return AGG_OPS;
    },
  },
  {
    name: "select-in-list",
    unit: "statements",
    ops: AGG_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      for (let index = 0; index < AGG_OPS; index += 1) {
        const result = await client.query(`SELECT COUNT(*) AS n FROM bench_items WHERE id IN (${IN_LIST_IDS})`);
        const matched = Number(cellAt(result, result.rows[0], 0));
        if (matched !== IN_LIST_ROWS) throw new Error(`in-list matched=${matched}, expected=${IN_LIST_ROWS}`);
      }
      return AGG_OPS;
    },
  },
  {
    name: "select-subquery-in",
    unit: "statements",
    ops: AGG_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      const expected = ROWS - SUBQUERY_BOUND;
      for (let index = 0; index < AGG_OPS; index += 1) {
        const result = await client.query(
          `SELECT COUNT(*) AS n FROM bench_items WHERE id IN (SELECT id FROM bench_items WHERE score > ${SUBQUERY_BOUND / 4})`,
        );
        const matched = Number(cellAt(result, result.rows[0], 0));
        if (matched !== expected) throw new Error(`subquery-in matched=${matched}, expected=${expected}`);
      }
      return AGG_OPS;
    },
  },
  {
    name: "select-exists",
    unit: "statements",
    ops: AGG_OPS * 2,
    async setup(client) {
      await seed(client, ROWS);
      await seedSparseSide(client, ROWS);
    },
    async run(client) {
      // Correlated `EXISTS` against the sparse side table: exactly the odd ids
      // hit, so both the positive and the negated form have fixed counts.
      for (let index = 0; index < AGG_OPS; index += 1) {
        const hit = await client.query(
          "SELECT COUNT(*) AS n FROM bench_items i WHERE EXISTS (SELECT 1 FROM bench_side_sparse s WHERE s.item_id = i.id)",
        );
        const matched = Number(cellAt(hit, hit.rows[0], 0));
        if (matched !== SIDE_HIT_ROWS) {
          throw new Error(`exists matched=${matched}, expected=${SIDE_HIT_ROWS}`);
        }
        const miss = await client.query(
          "SELECT COUNT(*) AS n FROM bench_items i WHERE NOT EXISTS (SELECT 1 FROM bench_side_sparse s WHERE s.item_id = i.id)",
        );
        const missing = Number(cellAt(miss, miss.rows[0], 0));
        if (missing !== SIDE_NULL_ROWS) {
          throw new Error(`not-exists matched=${missing}, expected=${SIDE_NULL_ROWS}`);
        }
      }
      return AGG_OPS * 2;
    },
  },
  {
    name: "select-union-all",
    unit: "statements",
    ops: AGG_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      const half = Math.floor(ROWS / 2);
      for (let index = 0; index < AGG_OPS; index += 1) {
        const result = await client.query(
          `SELECT id FROM bench_items WHERE id <= ${half} UNION ALL SELECT id FROM bench_items WHERE id > ${half}`,
        );
        if (result.rows.length !== ROWS) {
          throw new Error(`union-all rows=${result.rows.length}, expected ${ROWS}`);
        }
      }
      return AGG_OPS;
    },
  },
  {
    name: "select-cursor-page",
    unit: "pages",
    ops: Math.ceil(SCAN_ROWS / CURSOR_PAGE_SIZE),
    async setup(client) {
      await seed(client, SCAN_ROWS);
    },
    async run(client) {
      // Keyset pagination over the same 1,000 seeded rows as `select-scan`.
      // Each page is one statement; the unit is pages, not rows.
      const pages = Math.ceil(SCAN_ROWS / CURSOR_PAGE_SIZE);
      let last = 0;
      for (let page = 0; page < pages; page += 1) {
        const result = await client.query(
          `SELECT id, label FROM bench_items WHERE id > ${last} ORDER BY id LIMIT ${CURSOR_PAGE_SIZE}`,
        );
        const expected = Math.min(CURSOR_PAGE_SIZE, SCAN_ROWS - page * CURSOR_PAGE_SIZE);
        if (result.rows.length !== expected) {
          throw new Error(`cursor page ${page} rows=${result.rows.length}, expected=${expected}`);
        }
        last = Number(cellAt(result, result.rows[result.rows.length - 1], 0));
      }
      return pages;
    },
  },
  {
    name: "select-limit-offset",
    unit: "statements",
    ops: PAGE_OPS,
    async setup(client) {
      await seed(client, SCAN_ROWS);
    },
    async run(client) {
      // Deep OFFSET pagination over the same 1,000-row seed. The offset cycles
      // so both clients issue identical SQL and never leave the table.
      const span = SCAN_ROWS - 10;
      for (let index = 0; index < PAGE_OPS; index += 1) {
        const offset = (index * 37) % (span + 1);
        const result = await client.query(
          `SELECT id, label FROM bench_items ORDER BY id LIMIT 10 OFFSET ${offset}`,
        );
        if (result.rows.length !== 10) {
          throw new Error(`limit-offset offset=${offset} rows=${result.rows.length}`);
        }
        const first = Number(cellAt(result, result.rows[0], 0));
        if (first !== offset + 1) {
          throw new Error(`limit-offset offset=${offset} first=${first}`);
        }
      }
      return PAGE_OPS;
    },
  },
  {
    name: "order-index-desc",
    unit: "statements",
    ops: PAGE_OPS,
    async setup(client) {
      await seed(client, SCAN_ROWS);
    },
    async run(client) {
      // Same 1,000-row seed and the same cycled offsets as `select-limit-offset`,
      // but the index-backed column is walked backwards.
      const span = SCAN_ROWS - 10;
      for (let index = 0; index < PAGE_OPS; index += 1) {
        const offset = (index * 37) % (span + 1);
        const result = await client.query(
          `SELECT id, label FROM bench_items ORDER BY id DESC LIMIT 10 OFFSET ${offset}`,
        );
        if (result.rows.length !== 10) {
          throw new Error(`order-index-desc offset=${offset} rows=${result.rows.length}`);
        }
        const first = Number(cellAt(result, result.rows[0], 0));
        if (first !== SCAN_ROWS - offset) {
          throw new Error(`order-index-desc offset=${offset} first=${first}, expected=${SCAN_ROWS - offset}`);
        }
        const last = Number(cellAt(result, result.rows[result.rows.length - 1], 0));
        if (last !== SCAN_ROWS - offset - 9) {
          throw new Error(`order-index-desc offset=${offset} last=${last}, expected=${SCAN_ROWS - offset - 9}`);
        }
      }
      return PAGE_OPS;
    },
  },
  {
    name: "multi-table-join",
    unit: "statements",
    ops: JOIN_OPS,
    async setup(client) {
      await seed(client, ROWS);
      await seedSide(client, ROWS);
    },
    async run(client) {
      // Two physical tables joined on a non-key column; one side row per item
      // id keeps the result at 20 rows for both clients.
      for (let index = 0; index < JOIN_OPS; index += 1) {
        const result = await client.query(
          "SELECT b.label, s.weight FROM bench_items b JOIN bench_side s ON s.item_id = b.id WHERE b.id <= 20 ORDER BY b.id",
        );
        if (result.rows.length !== 20) {
          throw new Error(`multi-table-join rows=${result.rows.length}, expected=20`);
        }
      }
      return JOIN_OPS;
    },
  },
  {
    name: "left-join",
    unit: "statements",
    ops: JOIN_OPS,
    async setup(client) {
      await seed(client, ROWS);
      await seedSparseSide(client, ROWS);
    },
    async run(client) {
      // Same two-table shape as `multi-table-join`, but the side table is
      // sparse: every even item id must come back padded with NULL.
      for (let index = 0; index < JOIN_OPS; index += 1) {
        const result = await client.query(
          "SELECT i.id, s.weight FROM bench_items i LEFT JOIN bench_side_sparse s ON s.item_id = i.id ORDER BY i.id",
        );
        if (result.rows.length !== ROWS) {
          throw new Error(`left-join rows=${result.rows.length}, expected=${ROWS}`);
        }
        let nulls = 0;
        let previous = 0;
        for (const row of result.rows) {
          const id = Number(cellAt(result, row, 0));
          if (id !== previous + 1) throw new Error(`left-join id=${id}, expected=${previous + 1}`);
          previous = id;
          const weight = cellAt(result, row, 1);
          if (weight === null || weight === undefined) {
            nulls += 1;
            if (id % 2 !== 0) throw new Error(`left-join unexpected NULL id=${id}`);
          } else if (Number(weight) !== id / 4) {
            throw new Error(`left-join id=${id} weight=${weight}, expected=${id / 4}`);
          }
        }
        if (nulls !== SIDE_NULL_ROWS) {
          throw new Error(`left-join nulls=${nulls}, expected=${SIDE_NULL_ROWS}`);
        }
      }
      return JOIN_OPS;
    },
  },
  {
    name: "show-session",
    unit: "statements",
    ops: SHOW_OPS,
    async setup() {},
    async run(client) {
      // Session-command round trip: the synthesized variable table answers
      // without touching the executor. `SHOW STATUS` is not part of the
      // gateway surface (1064), so the equivalent round trip uses `SHOW
      // VARIABLES`.
      for (let index = 0; index < SHOW_OPS; index += 1) {
        const result = await client.query("SHOW VARIABLES LIKE 'version'");
        if (result.rows.length !== 1) {
          throw new Error(`show-session rows=${result.rows.length}`);
        }
      }
      return SHOW_OPS;
    },
  },
  {
    name: "select-null-blob",
    unit: "rows",
    ops: SCAN_ROWS,
    async setup(client) {
      await seedLarge(client, SCAN_ROWS);
    },
    async run(client) {
      // One statement over the same 1,000 row count as `select-scan`, with a
      // NULL every third row and a 2 KiB payload every fifth row.
      const result = await client.query("SELECT id, note, payload FROM bench_large ORDER BY id");
      if (result.rows.length !== SCAN_ROWS) {
        throw new Error(`${result.rows.length} rows, expected ${SCAN_ROWS}`);
      }
      return SCAN_ROWS;
    },
  },
  {
    name: "tx-commit",
    unit: "rows",
    ops: TX_COUNT * TX_ROWS,
    async setup(client) {
      await client.query(CREATE_TABLE);
    },
    async run(client) {
      // Same shape as the MySQL 8.4 comparison: N transactions of 10 inserts,
      // one COMMIT each.
      for (let tx = 0; tx < TX_COUNT; tx += 1) {
        await client.query("BEGIN");
        for (let row = 0; row < TX_ROWS; row += 1) {
          const id = tx * TX_ROWS + row + 1;
          await client.query(
            `INSERT INTO bench_items (id, label, score) VALUES (${id}, 'row-${id}', ${id / 4})`,
          );
        }
        await client.query("COMMIT");
      }
      return TX_COUNT * TX_ROWS;
    },
  },
  {
    name: "update-multi",
    unit: "statements",
    ops: MUTATE_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      // Each statement updates a four-row primary-key range; the range source
      // cycles over the same seed so both clients issue identical SQL.
      const groups = Math.max(1, Math.floor(ROWS / 4));
      for (let index = 0; index < MUTATE_OPS; index += 1) {
        const low = (index % groups) * 4 + 1;
        await client.query(
          `UPDATE bench_items SET label = 'upd-${index}' WHERE id BETWEEN ${low} AND ${low + 3}`,
        );
      }
      return MUTATE_OPS;
    },
  },
  {
    name: "delete-where",
    unit: "rows",
    ops: Math.min(MUTATE_OPS, Math.floor(ROWS / 2)) * 2,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      // Two-row range deletes walk the whole seed once; the unit is rows removed.
      const statements = Math.min(MUTATE_OPS, Math.floor(ROWS / 2));
      for (let index = 0; index < statements; index += 1) {
        const low = index * 2 + 1;
        await client.query(`DELETE FROM bench_items WHERE id BETWEEN ${low} AND ${low + 1}`);
      }
      return statements * 2;
    },
  },
  {
    name: "binary-prepared-rows",
    unit: "statements",
    ops: POINT_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      // Same point cardinality as `select-point`, but the row comes back over
      // the binary protocol with a prepare-time column description.
      const statement = await client.prepare("SELECT id, label FROM bench_items WHERE id = ?");
      try {
        for (let index = 0; index < POINT_OPS; index += 1) {
          const id = (index % ROWS) + 1;
          const result = await statement.execute([id]);
          if (result.rows.length !== 1) throw new Error(`binary point miss id=${id}`);
        }
      } finally {
        await statement.close();
      }
      return POINT_OPS;
    },
  },
  {
    name: "error-path",
    unit: "statements",
    ops: ERROR_OPS * 2,
    async setup(client) {
      await client.query(CREATE_TABLE);
      await client.query("INSERT INTO bench_items (id, label, score) VALUES (1, 'row-1', 0.25)");
    },
    async run(client) {
      // One duplicate-primary-key and one unknown-table round trip per
      // iteration; the connection must stay usable after both errors.
      for (let index = 0; index < ERROR_OPS; index += 1) {
        await expectError(
          client,
          "INSERT INTO bench_items (id, label, score) VALUES (1, 'dup', 0.25)",
          1062,
        );
        await expectError(client, "SELECT id FROM bench_missing WHERE id = 1", 1146);
      }
      return ERROR_OPS * 2;
    },
  },
  {
    name: "upsert-pk",
    unit: "statements",
    ops: ROWS,
    async setup(client) {
      await seedUnique(client, ROWS);
    },
    async run(client) {
      // `ON DUPLICATE KEY UPDATE` always hits the seeded primary key, so the row
      // count must not grow and `VALUES(label)` must win over the stored label.
      for (let id = 1; id <= ROWS; id += 1) {
        await client.query(
          `INSERT INTO bench_items_uk (id, uk, label, score) VALUES (${id}, ${id}, 'upd-${id}', ${id / 2}) ON DUPLICATE KEY UPDATE label = VALUES(label), score = VALUES(score)`,
        );
      }
      const result = await client.query("SELECT label, score FROM bench_items_uk WHERE id = 1");
      if (result.rows.length !== 1 || String(cellAt(result, result.rows[0], 0)) !== "upd-1") {
        throw new Error(`upsert-pk readback=${JSON.stringify(result.rows[0])}`);
      }
      return ROWS;
    },
  },
  {
    name: "insert-select",
    unit: "rows",
    ops: ROWS,
    async setup(client) {
      await seed(client, ROWS);
      await client.query("CREATE TABLE bench_copy (id INT PRIMARY KEY, label VARCHAR(40), score DOUBLE)");
    },
    async run(client) {
      await client.query("INSERT INTO bench_copy (id, label, score) SELECT id, label, score FROM bench_items");
      const result = await client.query("SELECT COUNT(*) AS n FROM bench_copy");
      const count = Number(cellAt(result, result.rows[0], 0));
      if (count !== ROWS) throw new Error(`insert-select rows=${count}, expected=${ROWS}`);
      return ROWS;
    },
  },
  {
    name: "alter-add-column",
    unit: "statements",
    ops: ALTER_OPS * 2,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      // Each iteration appends one column with a fixed DEFAULT and reads it
      // back for the row that already existed, so the DDL must rewrite the
      // stored rows rather than only the catalog.
      for (let index = 0; index < ALTER_OPS; index += 1) {
        const column = `c${index}`;
        const fallback = index % 7;
        await client.query(`ALTER TABLE bench_items ADD COLUMN ${column} INT DEFAULT ${fallback}`);
        const result = await client.query(`SELECT id, ${column} FROM bench_items WHERE id = 1`);
        if (result.rows.length !== 1) {
          throw new Error(`alter-add-column ${column} rows=${result.rows.length}`);
        }
        const value = Number(cellAt(result, result.rows[0], 1));
        if (value !== fallback) {
          throw new Error(`alter-add-column ${column} value=${value}, expected=${fallback}`);
        }
      }
      return ALTER_OPS * 2;
    },
  },
  {
    name: "tx-roundtrip",
    unit: "transactions",
    ops: TX_COUNT * 2,
    async setup() {},
    async run(client) {
      // One empty commit and one empty rollback per iteration.
      for (let index = 0; index < TX_COUNT; index += 1) {
        await client.query("BEGIN");
        await client.query("COMMIT");
        await client.query("BEGIN");
        await client.query("ROLLBACK");
      }
      return TX_COUNT * 2;
    },
  },
  {
    name: "window-row-number",
    unit: "statements",
    ops: WINDOW_OPS,
    async setup(client) {
      await seed(client, ROWS);
    },
    async run(client) {
      for (let index = 0; index < WINDOW_OPS; index += 1) {
        const result = await client.query(
          "SELECT id, ROW_NUMBER() OVER (ORDER BY id) AS rn FROM bench_items ORDER BY id LIMIT 20",
        );
        if (result.rows.length !== 20) throw new Error(`window-row-number rows=${result.rows.length}`);
        const first = Number(cellAt(result, result.rows[0], 1));
        if (first !== 1) throw new Error(`window-row-number first=${first}`);
      }
      return WINDOW_OPS;
    },
  },
  {
    name: "rollback-bulk",
    unit: "rows",
    ops: TX_COUNT * TX_ROWS,
    async setup(client) {
      await client.query(CREATE_TABLE);
    },
    async run(client) {
      // Same shape as `tx-commit`, but every transaction rolls back instead of
      // committing, so the store must discard the whole buffered batch.
      for (let tx = 0; tx < TX_COUNT; tx += 1) {
        await client.query("BEGIN");
        for (let row = 0; row < TX_ROWS; row += 1) {
          const id = row + 1;
          await client.query(
            `INSERT INTO bench_items (id, label, score) VALUES (${id}, 'row-${id}', ${id / 4})`,
          );
        }
        await client.query("ROLLBACK");
      }
      return TX_COUNT * TX_ROWS;
    },
  },
  {
    name: "savepoint-rollback",
    unit: "statements",
    ops: SAVEPOINT_OPS * 8,
    async setup(client) {
      await client.query("CREATE TABLE bench_sp (id INT PRIMARY KEY, val INT)");
    },
    async run(client) {
      // `ROLLBACK TO SAVEPOINT` 只撤销保存点之后的写入，保存点仍需可释放。
      for (let index = 0; index < SAVEPOINT_OPS; index += 1) {
        const base = index * 2;
        await client.query("BEGIN");
        await client.query(`INSERT INTO bench_sp VALUES (${base + 1}, ${index})`);
        await client.query("SAVEPOINT sp1");
        await client.query(`UPDATE bench_sp SET val = val + 1 WHERE id = ${base + 1}`);
        await client.query("ROLLBACK TO SAVEPOINT sp1");
        await client.query(`INSERT INTO bench_sp VALUES (${base + 2}, ${index})`);
        await client.query("RELEASE SAVEPOINT sp1");
        await client.query("COMMIT");
      }
      const count = await client.query("SELECT COUNT(*) FROM bench_sp");
      const kept = Number(cellAt(count, count.rows[0], 0));
      if (kept !== SAVEPOINT_OPS * 2) throw new Error(`savepoint-rollback rows=${kept}`);
      const rolledBack = await client.query("SELECT val FROM bench_sp WHERE id = 1");
      const value = Number(cellAt(rolledBack, rolledBack.rows[0], 0));
      if (value !== 0) throw new Error(`savepoint-rollback val=${value} expected=0`);
      return SAVEPOINT_OPS * 8;
    },
  },
];

// `BENCH_ONLY` keeps the report shape identical while letting a focused run
// measure a few workloads repeatedly.
const WORKLOADS =
  ONLY.length === 0
    ? ALL_WORKLOADS
    : ALL_WORKLOADS.filter((workload) => ONLY.includes(workload.name));
if (WORKLOADS.length === 0) {
  throw new Error(`BENCH_ONLY matched no workload: ${ONLY.join(", ")}`);
}

async function measure(workload, client) {
  const dataDir = await mkdtemp(join(tmpdir(), "mysql-bench-"));
  const server = startMysqlServer({ dataDir, port: 0 });
  let connection;
  try {
    connection = await client.connect(server.port);
    await workload.setup(connection);
    const start = process.hrtime.bigint();
    const ops = await workload.run(connection, () => client.connect(server.port));
    return { ms: elapsedMs(start), ops };
  } finally {
    if (connection !== undefined) await connection.close().catch(() => {});
    server.stop();
    await rm(dataDir, { recursive: true, force: true }).catch(() => {});
  }
}

function formatMs(value) {
  return value.toFixed(2);
}

function cell(value, fastest, formatted) {
  return fastest ? `**${formatted}**` : formatted;
}

async function main() {
  const samples = new Map();
  for (const workload of WORKLOADS) {
    const perClient = {};
    for (const client of CLIENTS) perClient[client.name] = [];
    samples.set(workload.name, perClient);
  }

  console.log("mysql vs-js-libraries benchmark");
  console.log(`Loopback 127.0.0.1, one text-protocol connection per client, fresh server + seed per measurement`);
  console.log(
    `Defaults: BENCH_ROWS=${ROWS}, BENCH_POINT=${POINT_OPS}, BENCH_SCAN=${SCAN_ROWS}, BENCH_ROUNDS=${ROUNDS}, BENCH_TX=${TX_COUNT}x${TX_ROWS}, BENCH_MUTATE=${MUTATE_OPS}, BENCH_ERROR=${ERROR_OPS}, BENCH_PAGE=${PAGE_OPS}, BENCH_JOIN=${JOIN_OPS}, BENCH_SHOW=${SHOW_OPS}, BENCH_BATCH_ROWS=${BATCH_ROWS}, BENCH_AGG=${AGG_OPS}, BENCH_CONNECT=${CONNECT_OPS}, BENCH_WINDOW=${WINDOW_OPS}, BENCH_SAVEPOINT=${SAVEPOINT_OPS}, BENCH_ALTER=${ALTER_OPS}`,
  );
  console.log("Warmup: 1 untimed round");
  if (ONLY.length > 0) console.log(`BENCH_ONLY: ${WORKLOADS.map((workload) => workload.name).join(", ")}`);
  for (const workload of WORKLOADS) {
    for (const client of CLIENTS) await measure(workload, client);
  }

  for (let round = 1; round <= ROUNDS; round += 1) {
    for (const workload of WORKLOADS) {
      for (const client of CLIENTS) {
        const { ms } = await measure(workload, client);
        samples.get(workload.name)[client.name].push(ms);
      }
    }
  }

  console.log("\nAbsolute (median per-round totals normalised per op)");
  console.log("| Workload | Unit | mysql2 ms/op | mysql2 ops/s | @yydb/mysql ms/op | @yydb/mysql ops/s |");
  console.log("|---------:|-----:|-------------:|-------------:|------------------:|------------------:|");
  for (const workload of WORKLOADS) {
    const baseline = median(samples.get(workload.name)["mysql2"]);
    const subject = median(samples.get(workload.name)["@yydb/mysql"]);
    console.log(
      `| ${workload.name} | ${workload.unit} | ${formatMs(baseline / workload.ops)} | ${throughput(workload.ops, baseline)} | ${formatMs(subject / workload.ops)} | ${throughput(workload.ops, subject)} |`,
    );
  }

  console.log("\nRelative to `mysql2` (ratio = @yydb/mysql_time / mysql2_time, 1.00x = parity)");
  console.log("| Workload | mysql2 | @yydb/mysql |");
  console.log("|---------:|-------:|------------:|");
  for (const workload of WORKLOADS) {
    const baseline = median(samples.get(workload.name)["mysql2"]);
    const subject = median(samples.get(workload.name)["@yydb/mysql"]);
    const ratio = subject / baseline;
    const subjectFastest = ratio < 1;
    const formattedRatio = `${ratio.toFixed(2)}x`;
    console.log(
      `| ${workload.name} | ${cell(1, !subjectFastest, "1.00x")} | ${cell(ratio, subjectFastest, formattedRatio)} |`,
    );
  }

  console.log("\np95 per-round total (ms)");
  console.log("| Workload | mysql2 | @yydb/mysql |");
  console.log("|---------:|-------:|------------:|");
  for (const workload of WORKLOADS) {
    const baseline = p95(samples.get(workload.name)["mysql2"]);
    const subject = p95(samples.get(workload.name)["@yydb/mysql"]);
    console.log(`| ${workload.name} | ${formatMs(baseline)} | ${formatMs(subject)} |`);
  }
}

try {
  await main();
} catch (error) {
  console.error("BENCHMARK FAIL");
  console.error(error);
  process.exitCode = 1;
}
