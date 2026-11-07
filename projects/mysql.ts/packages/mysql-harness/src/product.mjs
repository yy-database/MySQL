// Resolves the product entry point.
//
// In an install-state run `stages/install.mjs` sets `MYSQL_PRODUCT_ENTRY` to the
// installed `@yydb/mysql/dist/index.js`. In a workspace run it falls back to the
// sibling package build output.

import { existsSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';

export async function loadProduct() {
    const entry =
        process.env.MYSQL_PRODUCT_ENTRY ?? fileURLToPath(new URL('../../mysql/dist/index.js', import.meta.url));
    if (!existsSync(entry)) {
        throw new Error(`missing product build at ${entry}; run pnpm run build:ts first`);
    }
    return import(pathToFileURL(entry).href);
}
