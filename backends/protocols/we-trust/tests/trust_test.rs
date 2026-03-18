use bytes::{Bytes, BytesMut};
use futures::SinkExt;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use uuid::Uuid;
use we_trust::*;
use yykv_types::layout::{DsValueDecoder, DsValueEncoder};
use yykv_types::{DsError, DsValue, Redundancy};

#[test]
fn test_trust_header_signature() {
    let mut header = TrustHeader {
        version: 1,
        msg_type: MessageType::Kql as u8,
        flags: 0,
        length: 100,
        checksum: 12345,
        request_id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        signature: [0u8; 32],
    };

    let secret = b"my-secret-key";
    header.sign(secret);
    assert!(header.verify(secret));
    assert!(!header.verify(b"wrong-secret"));
}

#[test]
fn test_dsvalue_codec_basic() -> Result<(), DsError> {
    let tenant_id = Uuid::new_v4();
    let value = DsValue::Int(42);
    let encoded = DsValueCodec::encode(&value, tenant_id, Redundancy::SINGLE)?;

    let data = BytesMut::from(&encoded[..]);
    let (decoded, header) = DsValueCodec::decode(data)?;

    assert_eq!(decoded, value);
    assert_eq!(header.tenant_id, tenant_id);
    Ok(())
}

#[test]
fn test_dsvalue_codec_complex() -> Result<(), DsError> {
    let tenant_id = Uuid::new_v4();
    let mut dict = std::collections::BTreeMap::new();
    dict.insert("key1".to_string(), DsValue::Text("hello".into()));
    dict.insert("key2".to_string(), DsValue::Bool(true));
    let value = DsValue::Dict(dict);

    let encoded = DsValueCodec::encode(&value, tenant_id, Redundancy::TRUTH)?;

    let data = BytesMut::from(&encoded[..]);
    let (decoded, header) = DsValueCodec::decode(data)?;

    assert_eq!(decoded, value);
    assert_eq!(header.tenant_id, tenant_id);
    assert_eq!(header.sdr_level(), Redundancy::TRUTH);
    Ok(())
}

#[test]
fn test_dsvalue_codec_zero_copy_logic() -> Result<(), DsError> {
    let tenant_id = Uuid::new_v4();
    let value = DsValue::Text("zero-copy-test".repeat(100));

    let encoded = DsValueCodec::encode(&value, tenant_id, Redundancy::SINGLE)?;

    // The first TrustHeader::SIZE bytes should be the header
    assert!(encoded.len() > TrustHeader::SIZE);

    let mut header_bytes = BytesMut::from(&encoded[..TrustHeader::SIZE]);
    let header = TrustHeader::decode(&mut header_bytes).unwrap();
    assert_eq!(header.tenant_id, tenant_id);

    // The rest should be the encoded DsValue
    let payload = encoded.slice(TrustHeader::SIZE..);
    let decoded_value = DsValueDecoder::decode(&mut payload.into())?;
    assert_eq!(decoded_value, value);

    Ok(())
}

#[tokio::test]
async fn test_wetrust_client_server_integration() -> Result<(), DsError> {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| DsError::io_raw(e, None))?;
    let server_addr = listener
        .local_addr()
        .map_err(|e| DsError::io_raw(e, None))?;

    let tenant_id = Uuid::new_v4();
    let secret_key = b"test-secret-key".to_vec();
    let secret_key_clone = secret_key.clone();

    // Start a mock server
    tokio::spawn(async move {
        if let Ok((socket, _)) = listener.accept().await {
            let mut framed = Framed::new(socket, TrustCodec);

            // 1. Handle Auth
            if let Some(Ok(msg)) = framed.next().await
                && msg.header.msg_type == MessageType::Auth as u8
            {
                let mut resp = TrustMessage::new(
                    MessageType::Response,
                    msg.header.tenant_id,
                    Bytes::from("auth-ok"),
                );
                resp.header.sign(&secret_key_clone);
                let _ = framed.send(resp).await;
            }

            // 2. Handle PUT
            if let Some(Ok(msg)) = framed.next().await
                && msg.header.msg_type == MessageType::Put as u8
            {
                let mut resp =
                    TrustMessage::new(MessageType::Response, msg.header.tenant_id, Bytes::new());
                resp.header.sign(&secret_key_clone);
                let _ = framed.send(resp).await;
            }

            // 3. Handle GET
            if let Some(Ok(msg)) = framed.next().await
                && msg.header.msg_type == MessageType::Get as u8
            {
                let val = DsValue::Int(12345);
                let payload = DsValueEncoder::encode(&val).unwrap();
                let mut resp =
                    TrustMessage::new(MessageType::Response, msg.header.tenant_id, payload);
                resp.header.sign(&secret_key_clone);
                let _ = framed.send(resp).await;
            }
        }
    });

    // Client operations
    let mut client = WeTrustClient::connect(server_addr, tenant_id, secret_key).await?;

    // Test Put
    client.put("test_key", DsValue::Int(12345)).await?;

    // Test Get
    let val = client.get("test_key").await?;
    assert_eq!(val, Some(DsValue::Int(12345)));

    Ok(())
}
