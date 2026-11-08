import assert from "node:assert/strict";

process.env.PORT = "0";
process.env.MYSQL_EMBEDDED = "1";
delete process.env.MYSQL_PORT;

const { start } = await import("../src/server.mjs");

const running = await start();
const base = running.url;

async function api(method, path, body) {
  const response = await fetch(`${base}${path}`, {
    method,
    headers: body === undefined ? {} : { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let json = null;
  if (text.length > 0) {
    try {
      json = JSON.parse(text);
    } catch {
      json = text;
    }
  }
  return { status: response.status, body: json };
}

function pass(label) {
  console.log(`PASS ${label}`);
}

try {
  const health = await api("GET", "/health");
  assert.equal(health.status, 200);
  assert.deepEqual(health.body, { status: "ok" });
  pass("GET /health reaches SELECT 1");

  const created = await api("POST", "/products", { name: "widget", price: 9.5, stock: 10 });
  assert.equal(created.status, 201);
  assert.equal(created.body.name, "widget");
  assert.equal(created.body.stock, 10);
  const productId = created.body.id;
  pass("POST /products returns 201 with the inserted row");

  const duplicate = await api("POST", "/products", { id: productId, name: "duplicate", price: 1, stock: 1 });
  assert.equal(duplicate.status, 409, JSON.stringify(duplicate.body));
  pass("POST /products maps duplicate key 1062 to 409");

  const invalid = await api("POST", "/products", { name: "bad", price: -1, stock: 1 });
  assert.equal(invalid.status, 400);
  pass("POST /products validates the body");

  const second = await api("POST", "/products", { name: "gadget", price: 3, stock: 7 });
  assert.equal(second.status, 201);
  const secondId = second.body.id;
  pass("POST /products allocates a second auto-increment id");

  const listed = await api("GET", "/products?limit=2&offset=0");
  assert.equal(listed.status, 200);
  assert.equal(listed.body.products.length, 2);
  pass("GET /products applies limit and offset");

  const badPaging = await api("GET", "/products?limit=0");
  assert.equal(badPaging.status, 400);
  const badOffset = await api("GET", "/products?offset=-1");
  assert.equal(badOffset.status, 400);
  pass("GET /products rejects invalid pagination");

  const single = await api("GET", `/products/${productId}`);
  assert.equal(single.status, 200);
  assert.equal(single.body.id, productId);
  const missing = await api("GET", "/products/999");
  assert.equal(missing.status, 404);
  pass("GET /products/:id returns 200 and 404");

  const patched = await api("PATCH", `/products/${productId}`, { price: 12.5, stock: 4 });
  assert.equal(patched.status, 200);
  assert.equal(patched.body.price, 12.5);
  assert.equal(patched.body.stock, 4);
  const patchedMissing = await api("PATCH", "/products/999", { stock: 1 });
  assert.equal(patchedMissing.status, 404);
  pass("PATCH /products/:id returns 200 and 404");

  const order = await api("POST", "/orders", { productId, quantity: 2 });
  assert.equal(order.status, 201, JSON.stringify(order.body));
  assert.equal(order.body.quantity, 2);
  assert.equal(order.body.total, 25);
  assert.equal(order.body.remainingStock, 2);
  const afterOrder = await api("GET", `/products/${productId}`);
  assert.equal(afterOrder.body.stock, 2);
  pass("POST /orders decrements stock inside BEGIN/COMMIT");

  const insufficient = await api("POST", "/orders", { productId, quantity: 10 });
  assert.equal(insufficient.status, 409, JSON.stringify(insufficient.body));
  const afterReject = await api("GET", `/products/${productId}`);
  assert.equal(afterReject.body.stock, 2);
  const [orderRows] = await running.pool.query("SELECT COUNT(*) AS total FROM orders");
  assert.equal(Number(orderRows[0].total), 1);
  pass("POST /orders rolls back on insufficient stock (stock and order count unchanged)");

  const unknownOrder = await api("POST", "/orders", { productId: 999, quantity: 1 });
  assert.equal(unknownOrder.status, 404);
  pass("POST /orders returns 404 for an unknown product");

  const removed = await api("DELETE", `/products/${secondId}`);
  assert.equal(removed.status, 204);
  const removedAgain = await api("DELETE", `/products/${secondId}`);
  assert.equal(removedAgain.status, 404);
  pass("DELETE /products/:id returns 204 and 404");

  console.log("API TEST PASS");
} finally {
  await running.stop();
}
