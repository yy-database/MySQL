# with-express

An Express 5 REST API backed by `@yydb/mysql` through `mysql2`.

The example shows one honest integration path: `mysql2` provides the client
driver, while `startMysqlServer` provides the MySQL-compatible listener backed by
the product's `.yyds` / `.yykv` storage. The API adds validation, error mapping,
and a transaction that inserts an order and decrements stock.

## Prerequisites

From the repository root:

```shell
pnpm install
```

`express` and `mysql2` are root dev dependencies and resolve from the workspace
`node_modules`. Run `pnpm build` only when `packages/mysql/dist` or the platform
binding is missing. Examples are plain ESM and need no build step.

## Run

Embedded demo (default): starts the product server in-process on a free port and
a temporary data directory, then listens on `http://127.0.0.1:3000`.

```shell
pnpm --dir projects/mysql.ts/examples/with-express start
```

External server: point the API at an existing MySQL-compatible endpoint and skip
the embedded listener.

```shell
MYSQL_EMBEDDED=0 MYSQL_HOST=127.0.0.1 MYSQL_PORT=3306 MYSQL_USER=yydb MYSQL_PASSWORD= MYSQL_DATABASE=shop PORT=3000 pnpm --dir projects/mysql.ts/examples/with-express start
```

The schema and index are created at startup with `CREATE TABLE IF NOT EXISTS`
and a `CREATE INDEX` whose duplicate name error (1061) is ignored.

## Endpoints

| Method | Path | Result |
|--------|------|--------|
| `GET` | `/health` | `200` after `SELECT 1` |
| `GET` | `/products?limit=&offset=` | `200`; `400` for invalid pagination |
| `GET` | `/products/:id` | `200` or `404` |
| `POST` | `/products` | `201`; `400` validation; `409` duplicate key (1062 / `ER_DUP_ENTRY`) |
| `PATCH` | `/products/:id` | `200` or `404` |
| `DELETE` | `/products/:id` | `204` or `404` |
| `POST` | `/orders` | `201`; `400` validation; `404` unknown product; `409` insufficient stock |

`POST /products` accepts `name`, `price`, `stock`, and an optional explicit
`id`. The optional `id` makes the duplicate-key path directly reachable.

`POST /orders` takes `productId` and `quantity`. It runs `BEGIN`, inserts the
order, checks stock, writes the new literal stock value, then `COMMIT`s. When
stock is insufficient it `ROLLBACK`s the inserted order and returns `409`.

## Config

| Variable | Default | Notes |
|----------|---------|-------|
| `MYSQL_HOST` | `127.0.0.1` | external server host |
| `MYSQL_PORT` | `3306` | setting it also disables embedded mode |
| `MYSQL_USER` | `yydb` | product demo user |
| `MYSQL_PASSWORD` | empty | product demo password |
| `MYSQL_DATABASE` | `shop` | passed to the pool as the connection database |
| `MYSQL_EMBEDDED` | on | set to `0` for an external server |
| `PORT` | `3000` | HTTP listener |

## Test

```shell
pnpm --dir projects/mysql.ts/examples/with-express test
```

The test starts the embedded product server, listens on port `0`, and uses
`fetch` to assert `201/400/409/404/204`, the transaction stock decrement, and the
insufficient-stock rollback leaving both stock and order count unchanged.

## Key files

- `src/app.mjs` — `createApp({ pool })`, validation, error mapping, `ensureSchema`
- `src/server.mjs` — env config, pool `{connectionLimit: 4}`, embedded/external
  selection, graceful `SIGINT` / `SIGTERM` shutdown
- `tests/api.mjs` — end-to-end API assertions

## Product limitations observed

- `SELECT ... FOR UPDATE` is not parsed, so the order transaction reads stock
  before `BEGIN` and writes a computed literal value.
- `UPDATE products SET stock = stock - 1` is rejected with `ER_NOT_SUPPORTED_YET`
  ("write values must be simple literals"); the example reads the current stock
  and writes the new value instead.
- `CREATE DATABASE` and `USE` are not supported. `MYSQL_DATABASE` is still passed
  to the pool, but the product server exposes one implicit database.
- Transactions (`BEGIN` / `COMMIT` / `ROLLBACK`) are supported and verified by the
  test.
