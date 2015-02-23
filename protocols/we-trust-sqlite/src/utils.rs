//! SQLite 原生解析工具函数

use yykv_types::{DsError, DsValue};

type Result<T> = std::result::Result<T, DsError>;

/// 解析 SQLite 的变长整数 (Varint)
/// 返回 (解析出的值, 消耗的字节数)
pub fn parse_varint(data: &[u8]) -> (u64, usize) {
    let mut result: u64 = 0;
    for i in 0..8 {
        if i >= data.len() {
            return (result, i);
        }
        let byte = data[i];
        result = (result << 7) | (byte & 0x7F) as u64;
        if byte & 0x80 == 0 {
            return (result, i + 1);
        }
    }
    if data.len() > 8 {
        // 第 9 个字节使用全部 8 位
        result = (result << 8) | data[8] as u64;
        (result, 9)
    } else {
        (result, 8)
    }
}

/// 解析 SQLite Record 格式
pub fn parse_record(data: &[u8]) -> Result<Vec<DsValue>> {
    let (header_size, mut offset) = parse_varint(data);
    let header_end = header_size as usize;

    let mut serial_types = Vec::new();
    while offset < header_end {
        let (serial_type, consumed) = parse_varint(&data[offset..]);
        serial_types.push(serial_type);
        offset += consumed;
    }

    let mut values = Vec::new();
    let mut data_offset = header_end;

    for serial_type in serial_types {
        match serial_type {
            0 => values.push(DsValue::Null),
            1 => {
                values.push(DsValue::Int(data[data_offset] as i8 as i64));
                data_offset += 1;
            }
            2 => {
                let val = i16::from_be_bytes([data[data_offset], data[data_offset + 1]]);
                values.push(DsValue::Int(val as i64));
                data_offset += 2;
            }
            3 => {
                let val = ((data[data_offset] as i32) << 16)
                    | ((data[data_offset + 1] as i32) << 8)
                    | (data[data_offset + 2] as i32);
                let val = if val & 0x800000 != 0 {
                    val | !0xFFFFFF
                } else {
                    val
                };
                values.push(DsValue::Int(val as i64));
                data_offset += 3;
            }
            4 => {
                let val = i32::from_be_bytes([
                    data[data_offset],
                    data[data_offset + 1],
                    data[data_offset + 2],
                    data[data_offset + 3],
                ]);
                values.push(DsValue::Int(val as i64));
                data_offset += 4;
            }
            5 => {
                let val = ((data[data_offset] as i64) << 40)
                    | ((data[data_offset + 1] as i64) << 32)
                    | ((data[data_offset + 2] as i64) << 24)
                    | ((data[data_offset + 3] as i64) << 16)
                    | ((data[data_offset + 4] as i64) << 8)
                    | (data[data_offset + 5] as i64);
                let val = if val & 0x800000000000 != 0 {
                    val | !0xFFFFFFFFFFFF
                } else {
                    val
                };
                values.push(DsValue::Int(val));
                data_offset += 6;
            }
            6 => {
                let val = i64::from_be_bytes([
                    data[data_offset],
                    data[data_offset + 1],
                    data[data_offset + 2],
                    data[data_offset + 3],
                    data[data_offset + 4],
                    data[data_offset + 5],
                    data[data_offset + 6],
                    data[data_offset + 7],
                ]);
                values.push(DsValue::Int(val));
                data_offset += 8;
            }
            7 => {
                let val = f64::from_be_bytes([
                    data[data_offset],
                    data[data_offset + 1],
                    data[data_offset + 2],
                    data[data_offset + 3],
                    data[data_offset + 4],
                    data[data_offset + 5],
                    data[data_offset + 6],
                    data[data_offset + 7],
                ]);
                values.push(DsValue::Float(val));
                data_offset += 8;
            }
            8 => {
                values.push(DsValue::Int(0));
            }
            9 => {
                values.push(DsValue::Int(1));
            }
            t if t >= 12 && t % 2 == 0 => {
                let len = ((t - 12) / 2) as usize;
                let val = data[data_offset..data_offset + len].to_vec();
                values.push(DsValue::Bytes(val.into()));
                data_offset += len;
            }
            t if t >= 13 && t % 2 == 1 => {
                let len = ((t - 13) / 2) as usize;
                let val =
                    String::from_utf8_lossy(&data[data_offset..data_offset + len]).to_string();
                values.push(DsValue::Text(val));
                data_offset += len;
            }
            _ => {
                values.push(DsValue::Null);
            }
        }
    }

    Ok(values)
}

/// 比较两个 Value，遵循 SQLite 的比较规则:
/// NULL < INTEGER/REAL < TEXT < BLOB
pub fn compare_values(a: &DsValue, b: &DsValue) -> std::cmp::Ordering {
    use DsValue::*;
    use std::cmp::Ordering;

    match (a, b) {
        (Null, Null) => Ordering::Equal,
        (Null, _) => Ordering::Less,
        (_, Null) => Ordering::Greater,

        // 数字比较 (Int, Float, Decimal)
        (Int(v1), Int(v2)) => v1.cmp(v2),
        (Float(v1), Float(v2)) => v1.partial_cmp(v2).unwrap_or(Ordering::Equal),
        (Int(v1), Float(v2)) => (*v1 as f64).partial_cmp(v2).unwrap_or(Ordering::Equal),
        (Float(v1), Int(v2)) => v1.partial_cmp(&(*v2 as f64)).unwrap_or(Ordering::Equal),

        // Text 比较
        (Text(s1), Text(s2)) => s1.cmp(s2),

        // Bytes 比较
        (Bytes(b1), Bytes(b2))
        | (Binary(b1), Binary(b2))
        | (Bytes(b1), Binary(b2))
        | (Binary(b1), Bytes(b2)) => b1.cmp(b2),

        // 不同类型的比较规则: NULL < INTEGER/REAL < TEXT < BLOB
        (Int(_) | Float(_) | Decimal(_), Text(_)) => Ordering::Less,
        (Text(_), Int(_) | Float(_) | Decimal(_)) => Ordering::Greater,

        (Int(_) | Float(_) | Decimal(_) | Text(_), Bytes(_) | Binary(_)) => Ordering::Less,
        (Bytes(_) | Binary(_), Int(_) | Float(_) | Decimal(_) | Text(_)) => Ordering::Greater,

        // 默认实现
        _ => Ordering::Equal,
    }
}

/// 比较两个 Value 切片 (用于索引查找)
pub fn compare_value_slices(a: &[DsValue], b: &[DsValue]) -> std::cmp::Ordering {
    let len = std::cmp::min(a.len(), b.len());
    for i in 0..len {
        let cmp = compare_values(&a[i], &b[i]);
        if cmp != std::cmp::Ordering::Equal {
            return cmp;
        }
    }
    a.len().cmp(&b.len())
}

/// 将 u64 编码为 SQLite 的变长整数 (Varint)
pub fn encode_varint(mut value: u64) -> Vec<u8> {
    if value == 0 {
        return vec![0];
    }

    let mut res = Vec::new();
    if value > (1 << 56) - 1 {
        // 处理 9 字节情况
        res.push((value & 0xFF) as u8);
        value >>= 8;
        for _ in 0..8 {
            res.push(((value & 0x7F) | 0x80) as u8);
            value >>= 7;
        }
    } else {
        // 处理 1-8 字节情况
        res.push((value & 0x7F) as u8);
        value >>= 7;
        while value > 0 {
            res.push(((value & 0x7F) | 0x80) as u8);
            value >>= 7;
        }
    }
    res.reverse();
    res
}

/// 将 YYValue 列表编码为 SQLite Record 格式
pub fn encode_record(values: &[DsValue]) -> Vec<u8> {
    let mut header = Vec::new();
    let mut body = Vec::new();

    for value in values {
        match value {
            DsValue::Null => header.extend(encode_varint(0)),
            DsValue::Bool(b) => header.extend(encode_varint(if *b { 9 } else { 8 })),
            DsValue::Int(v) => {
                let v = *v;
                if v == 0 {
                    header.extend(encode_varint(8));
                } else if v == 1 {
                    header.extend(encode_varint(9));
                } else if (-128..=127).contains(&v) {
                    header.extend(encode_varint(1));
                    body.push(v as u8);
                } else if (-32768..=32767).contains(&v) {
                    header.extend(encode_varint(2));
                    body.extend_from_slice(&(v as i16).to_be_bytes());
                } else if (-8388608..=8388607).contains(&v) {
                    header.extend(encode_varint(3));
                    let b = (v as i32).to_be_bytes();
                    body.extend_from_slice(&b[1..]);
                } else if (-2147483648..=2147483647).contains(&v) {
                    header.extend(encode_varint(4));
                    body.extend_from_slice(&(v as i32).to_be_bytes());
                } else {
                    header.extend(encode_varint(6));
                    body.extend_from_slice(&v.to_be_bytes());
                }
            }
            DsValue::Float(v) => {
                header.extend(encode_varint(7));
                body.extend_from_slice(&v.to_be_bytes());
            }
            DsValue::Text(s) => {
                let len = s.len();
                header.extend(encode_varint((len * 2 + 13) as u64));
                body.extend_from_slice(s.as_bytes());
            }
            DsValue::Bytes(b) | DsValue::Binary(b) => {
                let len = b.len();
                header.extend(encode_varint((len * 2 + 12) as u64));
                body.extend_from_slice(b);
            }
            _ => {
                header.extend(encode_varint(0));
            }
        }
    }

    let mut record = encode_varint((header.len() + 1) as u64);
    record.extend(header);
    record.extend(body);
    record
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
