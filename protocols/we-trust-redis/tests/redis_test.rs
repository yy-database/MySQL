use bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use we_trust_redis::*;

#[test]
fn test_redis_codec_simple_string() {
    let mut codec = RedisCodec;
    let mut buf = BytesMut::from("+OK\r\n");
    let frame = codec.decode(&mut buf).unwrap().unwrap();
    assert_eq!(frame, RedisFrame::SimpleString("OK".to_string()));

    let mut out = BytesMut::new();
    codec
        .encode(RedisFrame::SimpleString("PONG".to_string()), &mut out)
        .unwrap();
    assert_eq!(out, "+PONG\r\n");
}

#[test]
fn test_redis_codec_bulk_string() {
    let mut codec = RedisCodec;
    let mut buf = BytesMut::from("$5\r\nhello\r\n");
    let frame = codec.decode(&mut buf).unwrap().unwrap();
    assert_eq!(
        frame,
        RedisFrame::BulkString(Some(Bytes::from_static(b"hello")))
    );

    let mut out = BytesMut::new();
    codec
        .encode(
            RedisFrame::BulkString(Some(Bytes::from_static(b"world"))),
            &mut out,
        )
        .unwrap();
    assert_eq!(out, "$5\r\nworld\r\n");
}

#[test]
fn test_redis_codec_decode() {
    let mut codec = RedisCodec;
    let mut buf = BytesMut::from("*2\r\n$4\r\necho\r\n$5\r\nhello\r\n");
    let frame = codec.decode(&mut buf).unwrap().unwrap();

    match frame {
        RedisFrame::Array(Some(frames)) => {
            assert_eq!(frames.len(), 2);
            assert_eq!(
                frames[0],
                RedisFrame::BulkString(Some(Bytes::from_static(b"echo")))
            );
            assert_eq!(
                frames[1],
                RedisFrame::BulkString(Some(Bytes::from_static(b"hello")))
            );
        }
        _ => panic!("Expected array"),
    }
}

#[test]
fn test_redis_command_parsing() {
    let frame = RedisFrame::Array(Some(vec![
        RedisFrame::BulkString(Some(Bytes::from_static(b"SET"))),
        RedisFrame::BulkString(Some(Bytes::from_static(b"key"))),
        RedisFrame::BulkString(Some(Bytes::from_static(b"value"))),
    ]));

    let cmd = RedisCommand::from_frame(frame).unwrap();
    match cmd {
        RedisCommand::Set(key, value) => {
            assert_eq!(key, "key");
            assert_eq!(value, &b"value"[..]);
        }
        _ => panic!("Expected SET command"),
    }
}
