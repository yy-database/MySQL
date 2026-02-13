use yyds_types::{Redundancy, YYValue, YYError};
use bytes::{BufMut, BytesMut, Bytes};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;
use we_trust::{TrustHeader, MessageType};
use crc32fast::Hasher;

type Result<T> = std::result::Result<T, YYError>;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:8889";
    println!("Connecting to YYDS server at {}...", addr);
    
    // 1. Test valid tenant (Nil UUID)
    let secret = b"yyds-secret-key-2026";
    test_tenant(addr, Uuid::nil(), "test_key", "test_value", secret).await?;
    
    // 3. Test MOUNT (.yydb file import)
    println!("\nTesting MOUNT (.yydb import)...");
    test_mount(addr, Uuid::nil(), secret).await?;
    
    // 2. Test invalid tenant
    println!("\nTesting invalid tenant...");
    let result = test_tenant(addr, Uuid::new_v4(), "invalid_key", "invalid_value", b"wrong_secret").await;
    match result {
        Ok(_) => println!("Error: Invalid tenant should have failed"),
        Err(e) => println!("Success: Invalid tenant failed as expected: {:?}", e),
    }

    Ok(())
}

async fn test_tenant(addr: &str, tenant_id: Uuid, key: &str, value: &str, secret: &[u8]) -> Result<()> {
    let mut stream = TcpStream::connect(addr).await
        .map_err(|e| YYError::io(format!("Failed to connect: {}", e)))?;
    println!("Connected for tenant {}", tenant_id);

    // --- PUT ---
    println!("Sending PUT request...");
    let val = YYValue::Text(value.to_string());
    let val_bytes = yyds_types::layout::DsValueEncoder::encode(&val)
        .map_err(|e| YYError::serialization(format!("Encode failed: {:?}", e)))?;
    
    let mut payload = BytesMut::new();
    payload.put_u32(key.len() as u32);
    payload.put_slice(key.as_bytes());
    payload.put_slice(&val_bytes);
    
    let mut hasher = Hasher::new();
    hasher.update(&payload);
    let checksum = hasher.finalize();

    let mut header = TrustHeader {
        version: 1,
        msg_type: MessageType::Put as u8,
        flags: 0,
        length: payload.len() as u32,
        checksum,
        request_id: Uuid::new_v4(),
        tenant_id,
        signature: [0u8; 32],
    };
    header.set_sdr_level(Redundancy::TRUTH);
    header.sign(secret);
    
    let mut header_bytes = BytesMut::with_capacity(TrustHeader::SIZE);
    header.encode(&mut header_bytes);
    
    stream.write_all(&header_bytes).await
        .map_err(|e| YYError::io(format!("Write failed: {}", e)))?;
    stream.write_all(&payload).await
        .map_err(|e| YYError::io(format!("Write failed: {}", e)))?;
    println!("PUT request sent.");

    // Read PUT response header
    let mut resp_header_buf = [0u8; TrustHeader::SIZE];
    stream.read_exact(&mut resp_header_buf).await
        .map_err(|e| YYError::io(format!("Read failed: {}", e)))?;
    let mut resp_header_bytes = BytesMut::from(&resp_header_buf[..]);
    let resp_header = TrustHeader::decode(&mut resp_header_bytes)
        .map_err(|e| YYError::serialization(format!("Decode failed: {:?}", e)))?;
    println!("PUT response header: msg_type={}, flags={}", resp_header.msg_type, resp_header.flags);
    if resp_header.flags != 0 {
        return Err(YYError::internal(format!("PUT failed with status {}", resp_header.flags)));
    }

    // --- GET ---
    println!("Sending GET request...");
    let mut get_payload = BytesMut::new();
    get_payload.put_u32(key.len() as u32);
    get_payload.put_slice(key.as_bytes());
    
    let mut hasher = Hasher::new();
    hasher.update(&get_payload);
    let checksum = hasher.finalize();

    let mut get_header = TrustHeader {
        version: 1,
        msg_type: MessageType::Get as u8,
        flags: 0,
        length: get_payload.len() as u32,
        checksum,
        request_id: Uuid::new_v4(),
        tenant_id,
        signature: [0u8; 32],
    };
    get_header.set_sdr_level(Redundancy::TRUTH);
    get_header.sign(secret);
    
    let mut get_header_bytes = BytesMut::with_capacity(TrustHeader::SIZE);
    get_header.encode(&mut get_header_bytes);
    
    stream.write_all(&get_header_bytes).await
        .map_err(|e| YYError::io(format!("Write failed: {}", e)))?;
    stream.write_all(&get_payload).await
        .map_err(|e| YYError::io(format!("Write failed: {}", e)))?;
    
    // Read GET response header
    let mut resp_header_buf = [0u8; TrustHeader::SIZE];
    stream.read_exact(&mut resp_header_buf).await
        .map_err(|e| YYError::io(format!("Read failed: {}", e)))?;
    let mut resp_header_bytes = BytesMut::from(&resp_header_buf[..]);
    let resp_header = TrustHeader::decode(&mut resp_header_bytes)
        .map_err(|e| YYError::serialization(format!("Decode failed: {:?}", e)))?;
    println!("GET response header: msg_type={}, flags={}, length={}", resp_header.msg_type, resp_header.flags, resp_header.length);

    if resp_header.flags != 0 {
        return Err(YYError::internal(format!("GET failed with status {}", resp_header.flags)));
    }

    if resp_header.length > 0 {
        let mut resp_data = vec![0u8; resp_header.length as usize];
        stream.read_exact(&mut resp_data).await
            .map_err(|e| YYError::io(format!("Read failed: {}", e)))?;
        let resp_bytes = Bytes::from(resp_data);
        let resp_val = yyds_types::layout::YYValueDecoder::decode(&resp_bytes)
            .map_err(|e| YYError::serialization(format!("Decode failed: {:?}", e)))?;
        println!("GET response: {:?}", resp_val);
        if let YYValue::Text(t) = resp_val {
            if t == value {
                println!("Value verification successful!");
            } else {
                return Err(YYError::internal(format!("Value mismatch: expected {}, got {}", value, t)));
            }
        }
    } else {
        return Err(YYError::internal("Expected response payload for valid tenant GET"));
    }

    // --- DELETE ---
    println!("Sending DELETE request...");
    let mut del_payload = BytesMut::new();
    del_payload.put_u32(key.len() as u32);
    del_payload.put_slice(key.as_bytes());

    let mut hasher = Hasher::new();
    hasher.update(&del_payload);
    let checksum = hasher.finalize();

    let mut del_header = TrustHeader {
        version: 1,
        msg_type: MessageType::Delete as u8,
        flags: 0,
        length: del_payload.len() as u32,
        checksum,
        request_id: Uuid::new_v4(),
        tenant_id,
        signature: [0u8; 32],
    };
    del_header.sign(secret);

    let mut del_header_bytes = BytesMut::with_capacity(TrustHeader::SIZE);
    del_header.encode(&mut del_header_bytes);

    stream.write_all(&del_header_bytes).await
        .map_err(|e| YYError::io(format!("Write failed: {}", e)))?;
    stream.write_all(&del_payload).await
        .map_err(|e| YYError::io(format!("Write failed: {}", e)))?;

    // Read DELETE response header
    let mut resp_header_buf = [0u8; TrustHeader::SIZE];
    stream.read_exact(&mut resp_header_buf).await
        .map_err(|e| YYError::io(format!("Read failed: {}", e)))?;
    let mut resp_header_bytes = BytesMut::from(&resp_header_buf[..]);
    let resp_header = TrustHeader::decode(&mut resp_header_bytes)
        .map_err(|e| YYError::serialization(format!("Decode failed: {:?}", e)))?;
    println!("DELETE response header: msg_type={}, flags={}", resp_header.msg_type, resp_header.flags);

    if resp_header.flags != 0 {
        return Err(YYError::internal(format!("DELETE failed with status {}", resp_header.flags)));
    }

    // --- GET again (should be 404) ---
    println!("Sending GET request after DELETE...");
    let mut hasher = Hasher::new();
    hasher.update(&del_payload);
    let checksum = hasher.finalize();

    let mut get_header2 = TrustHeader {
        version: 1,
        msg_type: MessageType::Get as u8,
        flags: 0,
        length: del_payload.len() as u32,
        checksum,
        request_id: Uuid::new_v4(),
        tenant_id,
        signature: [0u8; 32],
    };
    get_header2.sign(secret);

    let mut get_header_bytes2 = BytesMut::with_capacity(TrustHeader::SIZE);
    get_header2.encode(&mut get_header_bytes2);

    stream.write_all(&get_header_bytes2).await
        .map_err(|e| YYError::io(format!("Write failed: {}", e)))?;
    stream.write_all(&del_payload).await
        .map_err(|e| YYError::io(format!("Write failed: {}", e)))?;

    let mut resp_header_buf = [0u8; TrustHeader::SIZE];
    stream.read_exact(&mut resp_header_buf).await
        .map_err(|e| YYError::io(format!("Read failed: {}", e)))?;
    let mut resp_header_bytes = BytesMut::from(&resp_header_buf[..]);
    let resp_header = TrustHeader::decode(&mut resp_header_bytes)
        .map_err(|e| YYError::serialization(format!("Decode failed: {:?}", e)))?;
    println!("GET after DELETE response header: msg_type={}, flags={}", resp_header.msg_type, resp_header.flags);

    if resp_header.flags == 404 {
        println!("GET after DELETE returned 404 as expected.");
    } else {
        return Err(YYError::internal(format!("GET after DELETE expected 404, got {}", resp_header.flags)));
    }

    Ok(())
}

async fn test_mount(addr: &str, tenant_id: Uuid, secret: &[u8]) -> Result<()> {
    let mut stream = TcpStream::connect(addr).await
        .map_err(|e| YYError::io(format!("Failed to connect: {}", e)))?;
    
    // --- MOUNT ---
    let mut mount_payload = BytesMut::new();
    let file_path = "test.yydb";
    mount_payload.put_u32(file_path.len() as u32);
    mount_payload.put_slice(file_path.as_bytes());
    
    let mut hasher = Hasher::new();
    hasher.update(&mount_payload);
    let checksum = hasher.finalize();

    let mut mount_header = TrustHeader {
        version: 1,
        msg_type: MessageType::Rbq as u8, // Using Rbq for mount test
        flags: 0,
        length: mount_payload.len() as u32,
        checksum,
        request_id: Uuid::new_v4(),
        tenant_id,
        signature: [0u8; 32],
    };
    mount_header.sign(secret);
    
    let mut header_bytes = BytesMut::with_capacity(TrustHeader::SIZE);
    mount_header.encode(&mut header_bytes);
    
    stream.write_all(&header_bytes).await
        .map_err(|e| YYError::io(format!("Write failed: {}", e)))?;
    stream.write_all(&mount_payload).await
        .map_err(|e| YYError::io(format!("Write failed: {}", e)))?;
    
    let mut resp_header_buf = [0u8; TrustHeader::SIZE];
    stream.read_exact(&mut resp_header_buf).await
        .map_err(|e| YYError::io(format!("Read failed: {}", e)))?;
    let mut resp_header_bytes = BytesMut::from(&resp_header_buf[..]);
    let resp_header = TrustHeader::decode(&mut resp_header_bytes)
        .map_err(|e| YYError::serialization(format!("Decode failed: {:?}", e)))?;
    println!("MOUNT response header: msg_type={}, flags={}", resp_header.msg_type, resp_header.flags);
    
    if resp_header.flags != 0 {
        return Err(YYError::internal(format!("MOUNT failed with status {}", resp_header.flags)));
    }
    
    println!("MOUNT successful.");
    Ok(())
}
