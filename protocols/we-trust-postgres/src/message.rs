use bytes::{BufMut, BytesMut};
use yykv_types::DsValue;

#[derive(Debug, Clone)]
pub enum Message {
    Startup {
        params: Vec<(String, String)>,
    },
    SslRequest,
    Password(String),
    Query(String),
    Parse {
        name: String,
        query: String,
        param_types: Vec<u32>,
    },
    Bind {
        portal: String,
        statement: String,
        params: Vec<DsValue>,
    },
    Describe {
        target_type: u8, // 'S' for statement, 'P' for portal
        name: String,
    },
    Execute {
        portal: String,
        max_rows: i32,
    },
    Sync,
    Terminate,
    Close {
        target_type: u8, // 'S' for statement, 'P' for portal
        name: String,
    },
    Flush,
}

impl Message {
    pub fn encode(&self, dst: &mut BytesMut) {
        match self {
            Message::SslRequest => {
                dst.extend_from_slice(&8i32.to_be_bytes());
                dst.extend_from_slice(&80877103i32.to_be_bytes());
            }
            Message::Startup { params } => {
                let mut payload = BytesMut::new();
                payload.extend_from_slice(&196608i32.to_be_bytes());
                for (k, v) in params {
                    payload.extend_from_slice(k.as_bytes());
                    payload.extend_from_slice(&[0]);
                    payload.extend_from_slice(v.as_bytes());
                    payload.extend_from_slice(&[0]);
                }
                payload.extend_from_slice(&[0]);

                dst.extend_from_slice(&(payload.len() as i32 + 4).to_be_bytes());
                dst.extend_from_slice(&payload);
            }
            Message::Password(pass) => {
                dst.extend_from_slice(b"p");
                dst.extend_from_slice(&(pass.len() as i32 + 5).to_be_bytes());
                dst.extend_from_slice(pass.as_bytes());
                dst.extend_from_slice(&[0]);
            }
            Message::Query(q) => {
                dst.extend_from_slice(b"Q");
                dst.extend_from_slice(&(q.len() as i32 + 5).to_be_bytes());
                dst.extend_from_slice(q.as_bytes());
                dst.extend_from_slice(&[0]);
            }
            Message::Parse {
                name,
                query,
                param_types,
            } => {
                dst.extend_from_slice(b"P");
                let mut payload = BytesMut::new();
                payload.extend_from_slice(name.as_bytes());
                payload.extend_from_slice(&[0]);
                payload.extend_from_slice(query.as_bytes());
                payload.extend_from_slice(&[0]);
                payload.extend_from_slice(&(param_types.len() as i16).to_be_bytes());
                for &oid in param_types {
                    payload.extend_from_slice(&oid.to_be_bytes());
                }
                dst.extend_from_slice(&(payload.len() as i32 + 4).to_be_bytes());
                dst.extend_from_slice(&payload);
            }
            Message::Bind {
                portal,
                statement,
                params,
            } => {
                dst.extend_from_slice(b"B");
                let mut payload = BytesMut::new();
                payload.extend_from_slice(portal.as_bytes());
                payload.extend_from_slice(&[0]);
                payload.extend_from_slice(statement.as_bytes());
                payload.extend_from_slice(&[0]);

                // Parameter format codes (0 for text)
                payload.extend_from_slice(&0i16.to_be_bytes());

                // Parameter values
                payload.extend_from_slice(&(params.len() as i16).to_be_bytes());
                for param in params {
                    match param {
                        DsValue::Null => {
                            payload.extend_from_slice(&(-1i32).to_be_bytes());
                        }
                        _ => {
                            // Simplified for now: encode everything as string/text
                            let s = format!("{:?}", param);
                            payload.extend_from_slice(&(s.len() as i32).to_be_bytes());
                            payload.extend_from_slice(s.as_bytes());
                        }
                    }
                }

                // Result format codes (0 for text)
                payload.extend_from_slice(&0i16.to_be_bytes());

                dst.extend_from_slice(&(payload.len() as i32 + 4).to_be_bytes());
                dst.extend_from_slice(&payload);
            }
            Message::Describe { target_type, name } => {
                dst.extend_from_slice(b"D");
                let mut payload = BytesMut::new();
                payload.put_u8(*target_type);
                payload.extend_from_slice(name.as_bytes());
                payload.extend_from_slice(&[0]);
                dst.extend_from_slice(&(payload.len() as i32 + 4).to_be_bytes());
                dst.extend_from_slice(&payload);
            }
            Message::Execute { portal, max_rows } => {
                dst.extend_from_slice(b"E");
                let mut payload = BytesMut::new();
                payload.extend_from_slice(portal.as_bytes());
                payload.extend_from_slice(&[0]);
                payload.extend_from_slice(&max_rows.to_be_bytes());
                dst.extend_from_slice(&(payload.len() as i32 + 4).to_be_bytes());
                dst.extend_from_slice(&payload);
            }
            Message::Sync => {
                dst.extend_from_slice(b"S");
                dst.extend_from_slice(&4i32.to_be_bytes());
            }
            Message::Terminate => {
                dst.extend_from_slice(b"X");
                dst.extend_from_slice(&4i32.to_be_bytes());
            }
            Message::Close { target_type, name } => {
                dst.extend_from_slice(b"C");
                let mut payload = BytesMut::new();
                payload.put_u8(*target_type);
                payload.extend_from_slice(name.as_bytes());
                payload.extend_from_slice(&[0]);
                dst.extend_from_slice(&(payload.len() as i32 + 4).to_be_bytes());
                dst.extend_from_slice(&payload);
            }
            Message::Flush => {
                dst.extend_from_slice(b"H");
                dst.extend_from_slice(&4i32.to_be_bytes());
            }
        }
    }
}
