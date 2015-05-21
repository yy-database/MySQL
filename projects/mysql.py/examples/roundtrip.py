from mysql import decode_packet, encode_packet, max_packet_payload

framed = encode_packet(0, b"SELECT 1")
packet = decode_packet(framed)
print(max_packet_payload(), packet[0], packet[1])
