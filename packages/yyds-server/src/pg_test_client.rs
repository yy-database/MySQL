use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::BytesMut;
use yyds_types::YYError;
use we_trust_postgres::message::Message;

type Result<T> = std::result::Result<T, YYError>;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:5432";
    println!("Connecting to Postgres Gateway at {}...", addr);
    
    let mut stream = TcpStream::connect(addr).await
        .map_err(|e| YYError::io(format!("Failed to connect: {}", e)))?;
    println!("Connected.");

    // 1. Send Startup Message
    println!("Sending Startup Message...");
    let mut startup_buf = BytesMut::new();
    let startup_msg = Message::Startup { 
        params: vec![
            ("user".into(), "yyds".into()),
            ("database".into(), "yyds".into()),
        ] 
    };
    startup_msg.encode(&mut startup_buf);
    stream.write_all(&startup_buf).await
        .map_err(|e| YYError::io(format!("Failed to send startup: {}", e)))?;

    // 2. Read Response (should be ReadyForQuery 'I')
    // Ideally we should use PgCodec here but for simplicity we just read bytes
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await
        .map_err(|e| YYError::io(format!("Failed to read response: {}", e)))?;
    println!("Received {} bytes: {:?}", n, &buf[..n]);
    
    // Check for ReadyForQuery (Z) or AuthOk (R)
    // AuthOk: 'R' + len + 0
    // ReadyForQuery: 'Z' + len + status
    
    // 3. Send INSERT Query
    println!("Sending INSERT Query...");
    let mut query_buf = BytesMut::new();
    let query_msg = Message::Query("INSERT INTO test_table (id, name) VALUES (1, 'yyds')".into());
    query_msg.encode(&mut query_buf);
    stream.write_all(&query_buf).await
        .map_err(|e| YYError::io(format!("Failed to send query: {}", e)))?;

    // 4. Read Responses until ReadyForQuery (Z)
    loop {
        let mut head = [0u8; 5];
        stream.read_exact(&mut head).await
            .map_err(|e| YYError::io(format!("Failed to read response header: {}", e)))?;
        
        let tag = head[0];
        let len = i32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize;
        let mut body = vec![0u8; len - 4];
        stream.read_exact(&mut body).await
            .map_err(|e| YYError::io(format!("Failed to read response body: {}", e)))?;
        
        println!("Received Tag: '{}', Len: {}", tag as char, len);
        
        match tag {
            b'C' => {
                let complete_msg = String::from_utf8_lossy(&body[..body.len()-1]);
                println!("Command Complete: {}", complete_msg);
            }
            b'Z' => {
                println!("Ready for Query: {}", body[0] as char);
                break;
            }
            b'E' => {
                println!("Error Response: {:?}", String::from_utf8_lossy(&body));
                break;
            }
            _ => {
                println!("Other Message: {} bytes", body.len());
            }
        }
    }

    // 5. Send SELECT Query
    println!("\nSending SELECT Query...");
    let mut query_buf = BytesMut::new();
    let query_msg = Message::Query("SELECT * FROM test_table".into());
    query_msg.encode(&mut query_buf);
    stream.write_all(&query_buf).await
        .map_err(|e| YYError::io(format!("Failed to send query: {}", e)))?;

    loop {
        let mut head = [0u8; 5];
        stream.read_exact(&mut head).await
            .map_err(|e| YYError::io(format!("Failed to read response header: {}", e)))?;
        
        let tag = head[0];
        let len = i32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize;
        let mut body = vec![0u8; len - 4];
        stream.read_exact(&mut body).await
            .map_err(|e| YYError::io(format!("Failed to read response body: {}", e)))?;
        
        println!("Received Tag: '{}', Len: {}", tag as char, len);
        
        match tag {
            b'T' => println!("Row Description received"),
            b'D' => println!("Data Row received"),
            b'C' => {
                let complete_msg = String::from_utf8_lossy(&body[..body.len()-1]);
                println!("Command Complete: {}", complete_msg);
            }
            b'Z' => {
                println!("Ready for Query: {}", body[0] as char);
                break;
            }
            b'E' => {
                println!("Error Response: {:?}", String::from_utf8_lossy(&body));
                break;
            }
            _ => println!("Other Message: {} bytes", body.len()),
        }
    }

    Ok(())
}
