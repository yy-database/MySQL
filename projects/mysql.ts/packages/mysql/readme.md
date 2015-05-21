# @yydb/mysql

TypeScript acceptance surface for the MySQL classic-protocol primitives.

The package is a thin host: it re-exports the platform binding without
carrying its own engine or SQL layer. Parsing and execution live in
`mysql-core` on the Rust side.

```shell
pnpm run build
```
