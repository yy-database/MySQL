import express from "express";

const PRODUCT_COLUMNS = "id, name, price, stock";

function integer(value, { min = 0, max = Number.MAX_SAFE_INTEGER } = {}) {
  if (typeof value !== "number" || !Number.isInteger(value)) return null;
  if (value < min || value > max) return null;
  return value;
}

function integerParam(value, options) {
  if (typeof value !== "string" || !/^\d+$/.test(value)) return null;
  return integer(Number(value), options);
}

function name(value) {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  if (trimmed.length === 0 || trimmed.length > 80) return null;
  return trimmed;
}

function price(value) {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) return null;
  return value;
}

export async function ensureSchema(pool) {
  await pool.query(
    "CREATE TABLE IF NOT EXISTS products (id INT PRIMARY KEY AUTO_INCREMENT, name VARCHAR(80) NOT NULL, price DOUBLE NOT NULL, stock INT NOT NULL)",
  );
  await pool.query(
    "CREATE TABLE IF NOT EXISTS orders (id INT PRIMARY KEY AUTO_INCREMENT, product_id INT NOT NULL, quantity INT NOT NULL, total DOUBLE NOT NULL)",
  );
  try {
    await pool.query("CREATE INDEX idx_products_name ON products (name)");
  } catch (error) {
    // 1061 / ER_DUP_KEYNAME: the index already exists on a reused data directory.
    if (error.errno !== 1061 && error.code !== "ER_DUP_KEYNAME") throw error;
  }
}

function statusForError(error) {
  if (error.errno === 1062 || error.code === "ER_DUP_ENTRY") return 409;
  if (error.errno === 1146 || error.code === "ER_NO_SUCH_TABLE") return 404;
  if (error.errno === 1064 || error.errno === 1235) return 400;
  return 500;
}

export function createApp({ pool }) {
  const app = express();
  app.use(express.json());

  app.get("/health", async (_req, res) => {
    await pool.query("SELECT 1");
    res.json({ status: "ok" });
  });

  app.get("/products", async (req, res) => {
    const requestedLimit = req.query.limit === undefined ? 20 : integerParam(req.query.limit, { min: 1, max: 100 });
    const requestedOffset = req.query.offset === undefined ? 0 : integerParam(req.query.offset, { min: 0 });
    if (requestedLimit === null || requestedOffset === null) {
      res.status(400).json({ error: "invalid_pagination", message: "limit must be 1..100 and offset must be >= 0" });
      return;
    }
    const [rows] = await pool.query(
      `SELECT ${PRODUCT_COLUMNS} FROM products ORDER BY id LIMIT ? OFFSET ?`,
      [requestedLimit, requestedOffset],
    );
    res.json({ products: rows });
  });

  app.get("/products/:id", async (req, res) => {
    const id = integerParam(req.params.id, { min: 1 });
    if (id === null) {
      res.status(404).json({ error: "not_found", message: `product ${req.params.id} not found` });
      return;
    }
    const [rows] = await pool.query(`SELECT ${PRODUCT_COLUMNS} FROM products WHERE id = ?`, [id]);
    if (rows.length === 0) {
      res.status(404).json({ error: "not_found", message: `product ${id} not found` });
      return;
    }
    res.json(rows[0]);
  });

  app.post("/products", async (req, res) => {
    const body = req.body ?? {};
    const id = body.id === undefined ? null : integer(body.id, { min: 1 });
    const productName = name(body.name);
    const productPrice = price(body.price);
    const productStock = integer(body.stock, { min: 0 });
    if ((body.id !== undefined && id === null) || productName === null || productPrice === null || productStock === null) {
      res.status(400).json({
        error: "invalid_product",
        message: "name is required, price must be >= 0, stock must be a non-negative integer",
      });
      return;
    }

    const [result] = id === null
      ? await pool.query("INSERT INTO products (name, price, stock) VALUES (?, ?, ?)", [productName, productPrice, productStock])
      : await pool.query("INSERT INTO products (id, name, price, stock) VALUES (?, ?, ?, ?)", [id, productName, productPrice, productStock]);
    res.status(201).json({ id: id ?? result.insertId, name: productName, price: productPrice, stock: productStock });
  });

  app.patch("/products/:id", async (req, res) => {
    const id = integerParam(req.params.id, { min: 1 });
    const body = req.body ?? {};
    const assignments = [];
    const values = [];
    if (id === null) {
      res.status(404).json({ error: "not_found", message: `product ${req.params.id} not found` });
      return;
    }
    if ("name" in body) {
      const productName = name(body.name);
      if (productName === null) {
        res.status(400).json({ error: "invalid_product", message: "name must be 1..80 characters" });
        return;
      }
      assignments.push("name = ?");
      values.push(productName);
    }
    if ("price" in body) {
      const productPrice = price(body.price);
      if (productPrice === null) {
        res.status(400).json({ error: "invalid_product", message: "price must be >= 0" });
        return;
      }
      assignments.push("price = ?");
      values.push(productPrice);
    }
    if ("stock" in body) {
      const productStock = integer(body.stock, { min: 0 });
      if (productStock === null) {
        res.status(400).json({ error: "invalid_product", message: "stock must be a non-negative integer" });
        return;
      }
      assignments.push("stock = ?");
      values.push(productStock);
    }
    if (assignments.length === 0) {
      res.status(400).json({ error: "invalid_product", message: "provide at least one of name, price, stock" });
      return;
    }

    const [existing] = await pool.query("SELECT id FROM products WHERE id = ?", [id]);
    if (existing.length === 0) {
      res.status(404).json({ error: "not_found", message: `product ${id} not found` });
      return;
    }
    await pool.query(`UPDATE products SET ${assignments.join(", ")} WHERE id = ?`, [...values, id]);
    const [rows] = await pool.query(`SELECT ${PRODUCT_COLUMNS} FROM products WHERE id = ?`, [id]);
    res.json(rows[0]);
  });

  app.delete("/products/:id", async (req, res) => {
    const id = integerParam(req.params.id, { min: 1 });
    if (id === null) {
      res.status(404).json({ error: "not_found", message: `product ${req.params.id} not found` });
      return;
    }
    const [existing] = await pool.query("SELECT id FROM products WHERE id = ?", [id]);
    if (existing.length === 0) {
      res.status(404).json({ error: "not_found", message: `product ${id} not found` });
      return;
    }
    await pool.query("DELETE FROM products WHERE id = ?", [id]);
    res.status(204).end();
  });

  app.post("/orders", async (req, res) => {
    const body = req.body ?? {};
    const productId = integer(body.productId, { min: 1 });
    const quantity = integer(body.quantity, { min: 1 });
    if (productId === null || quantity === null) {
      res.status(400).json({ error: "invalid_order", message: "productId and quantity must be positive integers" });
      return;
    }

    const [products] = await pool.query("SELECT price, stock FROM products WHERE id = ?", [productId]);
    if (products.length === 0) {
      res.status(404).json({ error: "not_found", message: `product ${productId} not found` });
      return;
    }
    const { price: unitPrice, stock } = products[0];
    const total = unitPrice * quantity;

    const connection = await pool.getConnection();
    try {
      await connection.query("BEGIN");
      // Insert first so the insufficient-stock path exercises a real ROLLBACK.
      const [inserted] = await connection.query(
        "INSERT INTO orders (product_id, quantity, total) VALUES (?, ?, ?)",
        [productId, quantity, total],
      );
      if (stock < quantity) {
        await connection.query("ROLLBACK");
        res.status(409).json({
          error: "insufficient_stock",
          message: `product ${productId} has ${stock} in stock, requested ${quantity}`,
        });
        return;
      }
      await connection.query("UPDATE products SET stock = ? WHERE id = ?", [stock - quantity, productId]);
      await connection.query("COMMIT");
      res.status(201).json({ id: inserted.insertId, productId, quantity, total, remainingStock: stock - quantity });
    } catch (error) {
      await connection.query("ROLLBACK").catch(() => {});
      throw error;
    } finally {
      connection.release();
    }
  });

  app.use((_req, res) => {
    res.status(404).json({ error: "not_found", message: "unknown route" });
  });

  app.use((error, _req, res, _next) => {
    const status = error.status ?? statusForError(error);
    if (status >= 500) console.error(error);
    res.status(status).json({ error: error.code ?? "internal_error", message: error.message });
  });

  return app;
}
