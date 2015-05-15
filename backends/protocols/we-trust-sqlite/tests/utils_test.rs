use we_trust_sqlite::utils::*;

#[test]
fn test_varint() {
    let cases = vec![
        (0, vec![0x00]),
        (1, vec![0x01]),
        (127, vec![0x7F]),
        (128, vec![0x81, 0x00]),
        (16383, vec![0xFF, 0x7F]),
        (16384, vec![0x81, 0x80, 0x00]),
    ];

    for (val, expected) in cases {
        let encoded = encode_varint(val);
        assert_eq!(encoded, expected);
        let (decoded, size) = parse_varint(&encoded);
        assert_eq!(decoded, val);
        assert_eq!(size, encoded.len());
    }
}
