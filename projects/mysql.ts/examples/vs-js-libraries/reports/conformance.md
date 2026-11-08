# Conformance report

Command: `pnpm --dir projects/mysql.ts/examples/vs-js-libraries test`
Result: **PASS**

Captured output:

```text
PASS mysql2 write -> minimal client read (exact cells, ordered)
PASS minimal client write -> mysql2 read
PASS minimal client update -> mysql2 read
PASS minimal client delete -> mysql2 read
PASS ORDER BY id scan matches on both clients
CONFORMANCE PASS
```

The check starts one `startMysqlServer`, creates a table through `mysql2`, then
round-trips inserts, updates, deletes and an ordered scan across `mysql2` and the
hand-written classic-protocol client. Cell values are compared as text after
normalisation, so both drivers must agree on the exact stored value.
