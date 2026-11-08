import net from "node:net";

import { loadNative } from "../../../packages/mysql/dist/index.js";

// Hand-written classic-protocol client: handshake v10, mysql_native_password
// with an empty password, COM_QUERY, text results with EOF packets, COM_QUIT.
// Frames are produced by the product's native packet codec.

const CLIENT_LONG_PASSWORD = 0x00000001;
const CLIENT_LONG_FLAG = 0x00000004;
const CLIENT_TRANSACTIONS = 0x00002000;
const CLIENT_PROTOCOL_41 = 0x00000200;
const CLIENT_SECURE_CONNECTION = 0x00008000;
const CLIENT_MULTI_RESULTS = 0x00020000;
const CLIENT_PLUGIN_AUTH = 0x00080000;

const CLIENT_CAPABILITIES =
  CLIENT_LONG_PASSWORD |
  CLIENT_LONG_FLAG |
  CLIENT_TRANSACTIONS |
  CLIENT_PROTOCOL_41 |
  CLIENT_SECURE_CONNECTION |
  CLIENT_MULTI_RESULTS |
  CLIENT_PLUGIN_AUTH;

const COM_QUIT = 0x01;
const COM_QUERY = 0x03;

const OK_PACKET = 0x00;
const ERROR_PACKET = 0xff;
const EOF_PACKET = 0xfe;
const NULL_CELL = 0xfb;

const native = loadNative();

export class MysqlClientError extends Error {
  constructor(errno, code, sqlState, message) {
    super(message);
    this.name = "MysqlClientError";
    this.errno = errno;
    this.code = code;
    this.sqlState = sqlState;
  }
}

function lengthEncoded(buffer, cursor) {
  const first = buffer[cursor];
  if (first < 251) return { value: first, next: cursor + 1 };
  if (first === 252) return { value: buffer.readUInt16LE(cursor + 1), next: cursor + 3 };
  if (first === 253) return { value: buffer.readUIntLE(cursor + 1, 3), next: cursor + 4 };
  return { value: Number(buffer.readBigUInt64LE(cursor + 1)), next: cursor + 9 };
}

function lengthEncodedString(buffer, cursor) {
  const item = lengthEncoded(buffer, cursor);
  return {
    value: buffer.subarray(item.next, item.next + item.value).toString("utf8"),
    next: item.next + item.value,
  };
}

function readCString(buffer, cursor) {
  const end = buffer.indexOf(0, cursor);
  return { value: buffer.subarray(cursor, end).toString("utf8"), next: end + 1 };
}

function parseColumnName(payload) {
  // catalog, schema, table, org_table, name, ...
  let cursor = 0;
  for (let index = 0; index < 4; index += 1) {
    cursor = lengthEncodedString(payload, cursor).next;
  }
  return lengthEncodedString(payload, cursor).value;
}

function parseError(payload) {
  const errno = payload.readUInt16LE(1);
  let cursor = 3;
  let sqlState = "";
  if (payload[cursor] === 0x23) {
    sqlState = payload.subarray(cursor + 1, cursor + 6).toString("utf8");
    cursor += 6;
  }
  const message = payload.subarray(cursor).toString("utf8");
  return new MysqlClientError(errno, message, sqlState, message);
}

function parseRow(payload) {
  const cells = [];
  let cursor = 0;
  while (cursor < payload.length) {
    if (payload[cursor] === NULL_CELL) {
      cells.push(null);
      cursor += 1;
      continue;
    }
    const item = lengthEncoded(payload, cursor);
    cells.push(payload.subarray(item.next, item.next + item.value).toString("utf8"));
    cursor = item.next + item.value;
  }
  return cells;
}

function packetReader(socket) {
  let buffer = Buffer.alloc(0);
  const waiters = [];

  function pump() {
    while (waiters.length > 0) {
      if (buffer.length < 4) return;
      const length = buffer.readUIntLE(0, 3);
      if (buffer.length < 4 + length) return;
      const sequence = buffer[3];
      const payload = buffer.subarray(4, 4 + length);
      buffer = buffer.subarray(4 + length);
      waiters.shift()({ sequence, payload });
    }
  }

  socket.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    pump();
  });

  return () =>
    new Promise((resolve) => {
      waiters.push(resolve);
      pump();
    });
}

function buildHandshakeResponse(handshake) {
  const payload = handshake.payload;
  let cursor = 1;
  cursor = readCString(payload, cursor).next;
  cursor += 4; // connection id
  cursor += 8; // salt part 1
  cursor += 1; // filler
  const capabilitiesLow = payload.readUInt16LE(cursor);
  cursor += 2;
  cursor += 1; // charset
  cursor += 2; // status flags
  const capabilitiesHigh = payload.readUInt16LE(cursor);
  cursor += 2;
  const pluginNameLength = payload[cursor];
  cursor += 1;
  cursor += 10; // reserved
  cursor += 12; // salt part 2
  const pluginName = readCString(payload, cursor).value;
  const serverCapabilities = capabilitiesLow | (capabilitiesHigh << 16);
  const plugin = pluginNameLength > 0 || (serverCapabilities & CLIENT_PLUGIN_AUTH) !== 0 ? pluginName : "mysql_native_password";

  const response = Buffer.alloc(32 + 5 + 1 + 1 + 21);
  response.writeUInt32LE(CLIENT_CAPABILITIES >>> 0, 0);
  response.writeUInt32LE(0x01000000, 4); // max packet size
  response[8] = 45; // utf8mb4_general_ci
  response.fill(0, 9, 32);
  let at = 32;
  response.write("yydb", at, "utf8");
  at += 4;
  response[at++] = 0; // NUL terminator
  response[at++] = 0; // empty auth response for mysql_native_password
  response.write(plugin, at, "utf8");
  at += Buffer.byteLength(plugin, "utf8");
  response[at++] = 0; // NUL terminator
  return response.subarray(0, at);
}

export async function connectMysql(port, host = "127.0.0.1") {
  const socket = net.createConnection({ host, port });
  const read = packetReader(socket);
  const send = (sequence, payload) => socket.write(native.encodePacket(sequence, payload));

  await new Promise((resolve, reject) => {
    socket.once("error", reject);
    socket.once("connect", resolve);
  });

  const handshake = await read();
  send(handshake.sequence + 1, buildHandshakeResponse(handshake));
  const authResult = await read();
  if (authResult.payload[0] === ERROR_PACKET) throw parseError(authResult.payload);

  let closed = false;

  async function query(sql) {
    const request = Buffer.concat([Buffer.from([COM_QUERY]), Buffer.from(sql, "utf8")]);
    send(0, request);
    const first = await read();
    const header = first.payload[0];

    if (header === ERROR_PACKET) throw parseError(first.payload);
    if (header === OK_PACKET) {
      const affectedRows = lengthEncoded(first.payload, 1);
      const insertId = lengthEncoded(first.payload, affectedRows.next);
      return { columns: [], rows: [], affectedRows: Number(affectedRows.value), insertId: Number(insertId.value) };
    }

    const columnCount = lengthEncoded(first.payload, 0).value;
    const columns = [];
    for (let index = 0; index < columnCount; index += 1) {
      columns.push(parseColumnName((await read()).payload));
    }
    const columnTerminator = await read();
    if (columnTerminator.payload[0] !== EOF_PACKET) throw new Error("missing column terminator");

    const rows = [];
    while (true) {
      const packet = await read();
      const payload = packet.payload;
      if (payload[0] === EOF_PACKET && payload.length < 9) break;
      if (payload[0] === ERROR_PACKET) throw parseError(payload);
      rows.push(parseRow(payload));
    }
    return { columns, rows, affectedRows: rows.length, insertId: 0 };
  }

  async function close() {
    if (closed) return;
    closed = true;
    send(0, Buffer.from([COM_QUIT]));
    await new Promise((resolve) => {
      socket.once("close", resolve);
      socket.end();
    });
  }

  return { query, close };
}
