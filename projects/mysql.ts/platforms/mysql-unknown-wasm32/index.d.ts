export interface DecodedPacket {
    readonly sequence: number;
    readonly payload: Uint8Array;
}

export function max_packet_payload(): number;

export function encode_packet(sequence: number, payload: Uint8Array): Uint8Array;

export function decode_packet(input: Uint8Array): DecodedPacket | null;
