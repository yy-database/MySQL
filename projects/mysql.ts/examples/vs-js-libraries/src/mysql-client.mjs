import net from "node:net";

import { loadNative } from "../../../packages/mysql/dist/index.js";

// Hand-written classic-protocol client: handshake v10, mysql_native_password
// with an empty password, COM_QUERY, COM_STMT_PREPARE / COM_STMT_EXECUTE with
// text and binary rows, and COM_QUIT. Frames are produced by the product's
// native packet codec.

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
const COM_STMT_PREPARE = 0x16;
const COM_STMT_EXECUTE = 0x17;
const COM_STMT_CLOSE = 0x19;

const MYSQL_TYPE_TINY = 0x01;
const MYSQL_TYPE_SHORT = 0x02;
const MYSQL_TYPE_LONG = 0x03;
const MYSQL_TYPE_FLOAT = 0x04;
const MYSQL_TYPE_DOUBLE = 0x05;
const MYSQL_TYPE_NULL = 0x06;
const MYSQL_TYPE_LONGLONG = 0x08;
const MYSQL_TYPE_INT24 = 0x09;
const MYSQL_TYPE_DATE = 0x0a;
const MYSQL_TYPE_DATETIME = 0x0c;
const MYSQL_TYPE_TINY_BLOB = 0xf9;
const MYSQL_TYPE_MEDIUM_BLOB = 0xfa;
const MYSQL_TYPE_LONG_BLOB = 0xfb;
const MYSQL_TYPE_BLOB = 0xfc;
const MYSQL_TYPE_VAR_STRING = 0xfd;
const MYSQL_TYPE_STRING = 0xfe;

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

// ColumnDefinition: six length-encoded strings, a fixed-length marker (0x0C),
// then charset(2), column_length(4), type(1), flags(2), decimals(1).
function parseColumnDefinition(payload) {
  let cursor = 0;
  let name = "";
  for (let index = 0; index < 6; index += 1) {
    const item = lengthEncodedString(payload, cursor);
    if (index === 4) name = item.value;
    cursor = item.next;
  }
  cursor += 1 + 2 + 4;
  const type = payload[cursor];
  cursor += 1;
  const flags = payload.readUInt16LE(cursor);
  return { name, type, unsigned: (flags & 0x20) !== 0 };
}

function readBinaryValue(payload, cursor, type, unsigned) {
  switch (type) {
    case MYSQL_TYPE_TINY: {
      const raw = payload[cursor];
      return { value: unsigned ? raw : (raw << 24) >> 24, next: cursor + 1 };
    }
    case MYSQL_TYPE_SHORT: {
      const raw = payload.readUInt16LE(cursor);
      return { value: unsigned ? raw : (raw << 16) >> 16, next: cursor + 2 };
    }
    case MYSQL_TYPE_LONG: {
      const raw = payload.readUInt32LE(cursor);
      return { value: unsigned ? raw : raw | 0, next: cursor + 4 };
    }
    case MYSQL_TYPE_INT24: {
      const raw = payload.readUIntLE(cursor, 3);
      return { value: unsigned ? raw : (raw << 8) >> 8, next: cursor + 3 };
    }
    case MYSQL_TYPE_LONGLONG: {
      const raw = payload.readBigUInt64LE(cursor);
      return { value: unsigned ? Number(raw) : Number(BigInt.asIntN(64, raw)), next: cursor + 8 };
    }
    case MYSQL_TYPE_FLOAT:
      return { value: payload.readFloatLE(cursor), next: cursor + 4 };
    case MYSQL_TYPE_DOUBLE:
      return { value: payload.readDoubleLE(cursor), next: cursor + 8 };
    case MYSQL_TYPE_DATE:
    case MYSQL_TYPE_DATETIME:
      return readBinaryDate(payload, cursor);
    default: {
      const item = lengthEncoded(payload, cursor);
      const bytes = payload.subarray(item.next, item.next + item.value);
      const value = type === MYSQL_TYPE_BLOB || type === MYSQL_TYPE_TINY_BLOB || type === MYSQL_TYPE_MEDIUM_BLOB || type === MYSQL_TYPE_LONG_BLOB
        ? Buffer.from(bytes)
        : bytes.toString("utf8");
      return { value, next: item.next + item.value };
    }
  }
}

function readBinaryDate(payload, cursor) {
  const length = payload[cursor];
  if (length === 0) return { value: "0000-00-00", next: cursor + 1 };
  const year = payload.readUInt16LE(cursor + 1);
  const month = payload[cursor + 3];
  const day = payload[cursor + 4];
  let value = `${String(year).padStart(4, "0")}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
  if (length > 4) {
    const hour = payload[cursor + 5];
    const minute = payload[cursor + 6];
    const second = payload[cursor + 7];
    value += ` ${String(hour).padStart(2, "0")}:${String(minute).padStart(2, "0")}:${String(second).padStart(2, "0")}`;
  }
  return { value, next: cursor + 1 + length };
}

function parameterType(value) {
  if (typeof value === "number") return Number.isInteger(value) ? MYSQL_TYPE_LONGLONG : MYSQL_TYPE_DOUBLE;
  if (Buffer.isBuffer(value)) return MYSQL_TYPE_BLOB;
  return MYSQL_TYPE_VAR_STRING;
}

function encodeParameterValue(out, value) {
  if (typeof value === "number") {
    const bytes = Buffer.alloc(8);
    if (Number.isInteger(value)) bytes.writeBigInt64LE(BigInt(value));
    else bytes.writeDoubleLE(value);
    appendBytes(out, bytes);
    return;
  }
  if (Buffer.isBuffer(value)) {
    encodeLengthEncoded(out, value.length);
    appendBytes(out, value);
    return;
  }
  const bytes = Buffer.from(String(value), "utf8");
  encodeLengthEncoded(out, bytes.length);
  appendBytes(out, bytes);
}

function appendBytes(out, bytes) {
  for (let index = 0; index < bytes.length; index += 1) out.push(bytes[index]);
}

function encodeLengthEncoded(out, value) {
  if (value < 251) out.push(value);
  else if (value < 0x10000) out.push(252, value & 0xff, (value >> 8) & 0xff);
  else if (value < 0x1000000) out.push(253, value & 0xff, (value >> 8) & 0xff, (value >> 16) & 0xff);
  else {
    const bytes = Buffer.alloc(9);
    bytes[0] = 254;
    bytes.writeBigUInt64LE(BigInt(value), 1);
    out.push(...bytes);
  }
}

function parseBinaryRow(payload, columns) {
  const bitmapLength = Math.ceil((columns.length + 2) / 8);
  let cursor = 1 + bitmapLength;
  const row = [];
  for (let index = 0; index < columns.length; index += 1) {
    const bit = index + 2;
    if ((payload[1 + (bit >> 3)] & (1 << (bit & 7))) !== 0) {
      row.push(null);
      continue;
    }
    const item = readBinaryValue(payload, cursor, columns[index].type, columns[index].unsigned);
    row.push(item.value);
    cursor = item.next;
  }
  return row;
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

  async function prepare(sql) {
    const request = Buffer.concat([Buffer.from([COM_STMT_PREPARE]), Buffer.from(sql, "utf8")]);
    send(0, request);
    const first = await read();
    if (first.payload[0] === ERROR_PACKET) throw parseError(first.payload);
    if (first.payload[0] !== OK_PACKET) throw new Error("malformed COM_STMT_PREPARE response");
    const statementId = first.payload.readUInt32LE(1);
    const columnCount = first.payload.readUInt16LE(5);
    const parameterCount = first.payload.readUInt16LE(7);
    for (let index = 0; index < parameterCount; index += 1) await read();
    if (parameterCount > 0) await read(); // EOF after parameter definitions
    const columns = [];
    for (let index = 0; index < columnCount; index += 1) columns.push(parseColumnDefinition((await read()).payload));
    if (columnCount > 0) await read(); // EOF after column definitions

    async function execute(parameters = []) {
      const request = [COM_STMT_EXECUTE];
      const id = Buffer.alloc(4);
      id.writeUInt32LE(statementId);
      appendBytes(request, id);
      request.push(0); // flags
      request.push(1, 0, 0, 0); // iteration count
      if (parameterCount > 0) {
        const bitmap = Buffer.alloc(Math.ceil(parameterCount / 8));
        for (let index = 0; index < parameterCount; index += 1) {
          if (parameters[index] === null || parameters[index] === undefined) {
            bitmap[index >> 3] |= 1 << (index & 7);
          }
        }
        appendBytes(request, bitmap);
        request.push(1); // parameter types are bound on every execute
        for (let index = 0; index < parameterCount; index += 1) {
          const value = parameters[index];
          request.push(value === null || value === undefined ? MYSQL_TYPE_NULL : parameterType(value), 0);
        }
        for (let index = 0; index < parameterCount; index += 1) {
          const value = parameters[index];
          if (value === null || value === undefined) continue;
          encodeParameterValue(request, value);
        }
      }
      send(0, Buffer.from(request));
      const header = await read();
      const marker = header.payload[0];
      if (marker === ERROR_PACKET) throw parseError(header.payload);
      if (marker === OK_PACKET) {
        const affectedRows = lengthEncoded(header.payload, 1);
        const insertId = lengthEncoded(header.payload, affectedRows.next);
        return { columns: [], rows: [], affectedRows: Number(affectedRows.value), insertId: Number(insertId.value) };
      }
      const resultColumns = [];
      const resultTypes = [];
      for (let index = 0; index < columnCount; index += 1) {
        const definition = parseColumnDefinition((await read()).payload);
        resultColumns.push(definition.name);
        resultTypes.push(definition);
      }
      await read(); // EOF after column definitions
      const rows = [];
      while (true) {
        const packet = await read();
        const payload = packet.payload;
        if (payload[0] === EOF_PACKET && payload.length < 9) break;
        if (payload[0] === ERROR_PACKET) throw parseError(payload);
        rows.push(parseBinaryRow(payload, resultTypes));
      }
      return { columns: resultColumns, rows, affectedRows: rows.length, insertId: 0 };
    }

    function closeStatement() {
      const request = Buffer.alloc(5);
      request[0] = COM_STMT_CLOSE;
      request.writeUInt32LE(statementId, 1);
      send(0, request);
    }

    return { execute, close: closeStatement };
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

  return { query, prepare, close };
}
