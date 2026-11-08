import { createServer } from "node:http";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import mysql from "mysql2/promise";

import { startMysqlServer } from "../../../packages/mysql/dist/index.js";
import { createApp, ensureSchema } from "./app.mjs";

const PORT = Number(process.env.PORT ?? 3000);

async function startEmbedded() {
  const dataDir = await mkdtemp(join(tmpdir(), "mysql-express-"));
  const server = startMysqlServer({ dataDir, port: 0 });
  return { server, dataDir };
}

export async function start() {
  // Demo mode: no MYSQL_EMBEDDED=0 and no explicit MYSQL_PORT means the product
  // server is started in-process on a free port. Real deployments set the env
  // to an external MySQL-compatible server and skip the embedded listener.
  const embedded =
    process.env.MYSQL_EMBEDDED !== "0" && process.env.MYSQL_PORT === undefined ? await startEmbedded() : null;
  const host = process.env.MYSQL_HOST ?? "127.0.0.1";
  const port = embedded?.server.port ?? Number(process.env.MYSQL_PORT ?? 3306);
  const user = process.env.MYSQL_USER ?? "yydb";
  const password = process.env.MYSQL_PASSWORD ?? "";
  const database = process.env.MYSQL_DATABASE ?? "shop";

  const pool = mysql.createPool({ host, port, user, password, database, connectionLimit: 4 });
  await ensureSchema(pool);

  const httpServer = createServer(createApp({ pool }));
  await new Promise((resolve, reject) => {
    httpServer.once("error", reject);
    httpServer.listen(PORT, "127.0.0.1", resolve);
  });
  const address = httpServer.address();
  const url = `http://127.0.0.1:${typeof address === "object" && address !== null ? address.port : PORT}`;

  async function stop() {
    await new Promise((resolve) => httpServer.close(() => resolve()));
    await pool.end();
    if (embedded !== null) {
      embedded.server.stop();
      await rm(embedded.dataDir, { recursive: true, force: true }).catch(() => {});
    }
  }

  return { httpServer, pool, embedded, url, stop };
}

async function main() {
  const running = await start();
  console.log(`mysql with-express listening on ${running.url}`);
  if (running.embedded !== null) {
    console.log(`embedded product server on 127.0.0.1:${running.embedded.server.port} (data dir ${running.embedded.dataDir})`);
  } else {
    console.log("using external MySQL-compatible server from MYSQL_* environment variables");
  }

  let stopping = false;
  const shutdown = async (signal) => {
    if (stopping) return;
    stopping = true;
    console.log(`${signal} received, shutting down`);
    await running.stop();
    process.exit(0);
  };
  process.on("SIGINT", () => void shutdown("SIGINT"));
  process.on("SIGTERM", () => void shutdown("SIGTERM"));
}

if (process.argv[1] !== undefined && pathToFileURL(process.argv[1]).href === import.meta.url) {
  main().catch((error) => {
    console.error("failed to start with-express");
    console.error(error);
    process.exitCode = 1;
  });
}
