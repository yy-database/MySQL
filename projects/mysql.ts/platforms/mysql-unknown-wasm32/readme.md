Browser WebAssembly binding for `@yydb/mysql`.

ES modules with an explicit async init. The default export fetches the bundled
`mysql_wasm_bg.wasm` next to the glue; hosts without `fetch` pass the bytes
themselves.

```js
import init, { encode_packet, decode_packet } from '@yydb/mysql-unknown-wasm32';

await init(); // browser: loads ./mysql_wasm_bg.wasm relative to the glue
const framed = encode_packet(0, new TextEncoder().encode('SELECT 1'));
console.log(decode_packet(framed)?.sequence);

// Node or any host that reads the asset itself:
// await init({ module_or_path: readFileSync('lib/mysql_wasm_bg.wasm') });
```

The `.wasm` and its glue are produced by `wasm-pack` from
`projects/mysql.rs/mysql-wasm` into `lib/` and are not committed.
