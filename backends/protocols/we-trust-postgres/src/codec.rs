use crate::Result;
use crate::message::Message;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use yykv_types::DsError;

#[derive(Debug, Clone)]
pub enum BackendMessage {
    AuthenticationOk,
    AuthenticationCleartextPassword,
    AuthenticationMD5Password { salt: [u8; 4] },
    ParameterStatus { name: String, value: String },
    BackendKeyData { process_id: i32, secret_key: i32 },
    ReadyForQuery { status: u8 },
    RowDescription { fields: Vec<FieldDescription> },
    DataRow { values: Vec<Option<Bytes>> },
    CommandComplete { tag: String },
    ErrorResponse { fields: Vec<(u8, String)> },
    NoticeResponse { fields: Vec<(u8, String)> },
    ParseComplete,
    BindComplete,
    NoData,
    ParameterDescription { ids: Vec<u32> },
    CloseComplete,
}

#[derive(Debug, Clone)]
pub struct FieldDescription {
    pub name: String,
    pub table_oid: i32,
    pub column_id: i16,
    pub type_oid: i32,
    pub type_size: i16,
    pub type_modifier: i32,
    pub format_code: i16,
}

#[derive(Debug, Clone, Default)]
pub struct PgCodec;

impl PgCodec {
    pub fn new() -> Self {
        Self
    }
}

pub struct PgServerCodec;

impl Default for PgServerCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl PgServerCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for PgServerCodec {
    type Item = Message;
    type Error = DsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.is_empty() {
            return Ok(None);
        }

        // Handle Startup/SSLRequest (no tag byte)
        if src.len() >= 4 {
            let len = i32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;
            if len == 8 && src.len() >= 8 {
                let code = i32::from_be_bytes([src[4], src[5], src[6], src[7]]);
                if code == 80877103 {
                    src.advance(8);
                    return Ok(Some(Message::SslRequest));
                }
            }

            // Startup Message: length (i32) + protocol (i32) + parameters (strings)
            if len > 8 && src.len() >= len {
                let protocol = i32::from_be_bytes([src[4], src[5], src[6], src[7]]);
                if protocol == 196608 {
                    // 3.0
                    let mut body = src.split_to(len);
                    body.advance(8);
                    let mut params = Vec::new();
                    while body[0] != 0 {
                        let name = PgCodec::read_string(&mut body)?;
                        let value = PgCodec::read_string(&mut body)?;
                        params.push((name, value));
                    }
                    return Ok(Some(Message::Startup { params }));
                }
            }
        }

        if src.len() < 5 {
            return Ok(None);
        }

        let tag = src[0];
        let len = i32::from_be_bytes([src[1], src[2], src[3], src[4]]) as usize;

        if src.len() < len + 1 {
            return Ok(None);
        }

        let mut body = src.split_to(len + 1);
        body.advance(5);

        match tag {
            b'p' => {
                let pass = PgCodec::read_string(&mut body)?;
                Ok(Some(Message::Password(pass)))
            }
            b'Q' => {
                let query = PgCodec::read_string(&mut body)?;
                Ok(Some(Message::Query(query)))
            }
            b'P' => {
                let name = PgCodec::read_string(&mut body)?;
                let query = PgCodec::read_string(&mut body)?;
                let num_params = body.get_i16();
                let mut param_types = Vec::with_capacity(num_params as usize);
                for _ in 0..num_params {
                    param_types.push(body.get_u32());
                }
                Ok(Some(Message::Parse {
                    name,
                    query,
                    param_types,
                }))
            }
            b'B' => {
                let portal = PgCodec::read_string(&mut body)?;
                let statement = PgCodec::read_string(&mut body)?;

                let num_format_codes = body.get_i16();
                let mut format_codes = Vec::with_capacity(num_format_codes as usize);
                for _ in 0..num_format_codes {
                    format_codes.push(body.get_i16());
                }

                let num_params = body.get_i16();
                let mut params = Vec::with_capacity(num_params as usize);
                for i in 0..num_params {
                    let len = body.get_i32();
                    if len == -1 {
                        params.push(yykv_types::DsValue::Null);
                    } else {
                        let mut val_bytes = vec![0u8; len as usize];
                        body.copy_to_slice(&mut val_bytes);

                        let format = if format_codes.len() == 1 {
                            format_codes[0]
                        } else if format_codes.len() > i as usize {
                            format_codes[i as usize]
                        } else {
                            0 // default to text
                        };

                        if format == 0 {
                            // Text
                            let s = String::from_utf8_lossy(&val_bytes).to_string();
                            params.push(yykv_types::DsValue::Text(s));
                        } else {
                            // Binary
                            params.push(yykv_types::DsValue::Bytes(val_bytes.into()));
                        }
                    }
                }
                Ok(Some(Message::Bind {
                    portal,
                    statement,
                    params,
                }))
            }
            b'D' => {
                let target_type = body.get_u8();
                let name = PgCodec::read_string(&mut body)?;
                Ok(Some(Message::Describe { target_type, name }))
            }
            b'C' => {
                let target_type = body.get_u8();
                let name = PgCodec::read_string(&mut body)?;
                Ok(Some(Message::Close { target_type, name }))
            }
            b'E' => {
                let portal = PgCodec::read_string(&mut body)?;
                let max_rows = body.get_i32();
                Ok(Some(Message::Execute { portal, max_rows }))
            }
            b'S' => Ok(Some(Message::Sync)),
            b'H' => Ok(Some(Message::Flush)),
            b'X' => Ok(Some(Message::Terminate)),
            _ => {
                // Skip unknown messages for now to avoid hanging
                Ok(None)
            }
        }
    }
}

impl Encoder<BackendMessage> for PgServerCodec {
    type Error = DsError;

    fn encode(&mut self, item: BackendMessage, dst: &mut BytesMut) -> Result<()> {
        item.encode(dst);
        Ok(())
    }
}

impl BackendMessage {
    pub fn encode(&self, dst: &mut BytesMut) {
        match self {
            BackendMessage::AuthenticationOk => {
                dst.put_u8(b'R');
                dst.put_i32(8);
                dst.put_i32(0);
            }
            BackendMessage::ReadyForQuery { status } => {
                dst.put_u8(b'Z');
                dst.put_i32(5);
                dst.put_u8(*status);
            }
            BackendMessage::CommandComplete { tag } => {
                dst.put_u8(b'C');
                let len = tag.len() + 1 + 4;
                dst.put_i32(len as i32);
                dst.put_slice(tag.as_bytes());
                dst.put_u8(0);
            }
            BackendMessage::ParameterStatus { name, value } => {
                dst.put_u8(b'S');
                let len = name.len() + value.len() + 2 + 4;
                dst.put_i32(len as i32);
                dst.put_slice(name.as_bytes());
                dst.put_u8(0);
                dst.put_slice(value.as_bytes());
                dst.put_u8(0);
            }
            BackendMessage::RowDescription { fields } => {
                dst.put_u8(b'T');
                let mut payload = BytesMut::new();
                payload.put_i16(fields.len() as i16);
                for field in fields {
                    payload.put_slice(field.name.as_bytes());
                    payload.put_u8(0);
                    payload.put_i32(field.table_oid);
                    payload.put_i16(field.column_id);
                    payload.put_i32(field.type_oid);
                    payload.put_i16(field.type_size);
                    payload.put_i32(field.type_modifier);
                    payload.put_i16(field.format_code);
                }
                dst.put_i32(payload.len() as i32 + 4);
                dst.extend_from_slice(&payload);
            }
            BackendMessage::DataRow { values } => {
                dst.put_u8(b'D');
                let mut payload = BytesMut::new();
                payload.put_i16(values.len() as i16);
                for val in values {
                    match val {
                        Some(v) => {
                            payload.put_i32(v.len() as i32);
                            payload.put_slice(v);
                        }
                        None => {
                            payload.put_i32(-1);
                        }
                    }
                }
                dst.put_i32(payload.len() as i32 + 4);
                dst.extend_from_slice(&payload);
            }
            BackendMessage::ErrorResponse { fields } => {
                dst.put_u8(b'E');
                let mut payload = BytesMut::new();
                for (tag, msg) in fields {
                    payload.put_u8(*tag);
                    payload.put_slice(msg.as_bytes());
                    payload.put_u8(0);
                }
                payload.put_u8(0);
                dst.put_i32(payload.len() as i32 + 4);
                dst.extend_from_slice(&payload);
            }
            BackendMessage::ParameterDescription { ids } => {
                dst.put_u8(b't');
                let mut payload = BytesMut::new();
                payload.put_i16(ids.len() as i16);
                for &id in ids {
                    payload.put_u32(id);
                }
                dst.put_i32(payload.len() as i32 + 4);
                dst.extend_from_slice(&payload);
            }
            BackendMessage::CloseComplete => {
                dst.put_u8(b'3');
                dst.put_i32(4);
            }
            BackendMessage::ParseComplete => {
                dst.put_u8(b'1');
                dst.put_i32(4);
            }
            BackendMessage::BindComplete => {
                dst.put_u8(b'2');
                dst.put_i32(4);
            }
            BackendMessage::NoData => {
                dst.put_u8(b'n');
                dst.put_i32(4);
            }
            _ => {
                // TODO: Implement other backend messages
            }
        }
    }
}

impl Decoder for PgCodec {
    type Item = BackendMessage;
    type Error = DsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.len() < 5 {
            return Ok(None);
        }

        let tag = src[0];
        let len = i32::from_be_bytes([src[1], src[2], src[3], src[4]]) as usize;

        if src.len() < len + 1 {
            return Ok(None);
        }

        let mut body = src.split_to(len + 1);
        body.advance(5); // Skip tag and length

        let msg = match tag {
            b'R' => Self::decode_authentication(&mut body)?,
            b'S' => {
                let name = Self::read_string(&mut body)?;
                let value = Self::read_string(&mut body)?;
                BackendMessage::ParameterStatus { name, value }
            }
            b'K' => {
                let process_id = body.get_i32();
                let secret_key = body.get_i32();
                BackendMessage::BackendKeyData {
                    process_id,
                    secret_key,
                }
            }
            b'Z' => {
                let status = body.get_u8();
                BackendMessage::ReadyForQuery { status }
            }
            b'T' => Self::decode_row_description(&mut body)?,
            b'D' => Self::decode_data_row(&mut body)?,
            b'C' => {
                let tag = Self::read_string(&mut body)?;
                BackendMessage::CommandComplete { tag }
            }
            b'E' => BackendMessage::ErrorResponse {
                fields: Self::decode_error_notice(&mut body)?,
            },
            b'N' => BackendMessage::NoticeResponse {
                fields: Self::decode_error_notice(&mut body)?,
            },
            b'1' => BackendMessage::ParseComplete,
            b'2' => BackendMessage::BindComplete,
            b'n' => BackendMessage::NoData,
            b't' => {
                let count = body.get_i16();
                let mut ids = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    ids.push(body.get_u32());
                }
                BackendMessage::ParameterDescription { ids }
            }
            b'3' => BackendMessage::CloseComplete,
            _ => {
                return Err(DsError::protocol(format!(
                    "Unknown backend tag: {}",
                    tag as char
                )));
            }
        };

        Ok(Some(msg))
    }
}

impl Encoder<Message> for PgCodec {
    type Error = DsError;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<()> {
        item.encode(dst);
        Ok(())
    }
}

impl PgCodec {
    fn decode_authentication(body: &mut BytesMut) -> Result<BackendMessage> {
        let auth_type = body.get_i32();
        match auth_type {
            0 => Ok(BackendMessage::AuthenticationOk),
            3 => Ok(BackendMessage::AuthenticationCleartextPassword),
            5 => {
                let mut salt = [0u8; 4];
                body.copy_to_slice(&mut salt);
                Ok(BackendMessage::AuthenticationMD5Password { salt })
            }
            _ => Err(DsError::protocol(format!(
                "Unsupported authentication type: {}",
                auth_type
            ))),
        }
    }

    fn decode_row_description(body: &mut BytesMut) -> Result<BackendMessage> {
        let count = body.get_i16();
        let mut fields = Vec::with_capacity(count as usize);
        for _ in 0..count {
            fields.push(FieldDescription {
                name: Self::read_string(body)?,
                table_oid: body.get_i32(),
                column_id: body.get_i16(),
                type_oid: body.get_i32(),
                type_size: body.get_i16(),
                type_modifier: body.get_i32(),
                format_code: body.get_i16(),
            });
        }
        Ok(BackendMessage::RowDescription { fields })
    }

    fn decode_data_row(body: &mut BytesMut) -> Result<BackendMessage> {
        let count = body.get_i16();
        let mut values = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let len = body.get_i32();
            if len == -1 {
                values.push(None);
            } else {
                values.push(Some(body.split_to(len as usize).freeze()));
            }
        }
        Ok(BackendMessage::DataRow { values })
    }

    fn decode_error_notice(body: &mut BytesMut) -> Result<Vec<(u8, String)>> {
        let mut fields = Vec::new();
        loop {
            let tag = body.get_u8();
            if tag == 0 {
                break;
            }
            fields.push((tag, Self::read_string(body)?));
        }
        Ok(fields)
    }

    fn read_string(body: &mut BytesMut) -> Result<String> {
        let pos = body
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| DsError::protocol("String not null-terminated"))?;
        let s = String::from_utf8_lossy(&body[..pos]).into_owned();
        body.advance(pos + 1);
        Ok(s)
    }
}
