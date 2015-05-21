import { loadNative } from '@yydb/mysql';

const native = loadNative();
const framed = native.encodePacket(0, Buffer.from('SELECT 1'));
const packet = native.decodePacket(framed);
if (packet === null) throw new Error('the binding returned no packet');
console.log(`sequence=${packet.sequence} payload=${packet.payload.toString()}`);
