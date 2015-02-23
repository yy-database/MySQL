use bytes::{Buf, BufMut, BytesMut};
use oak_json::JsonValue;
use serde::{Deserialize, Serialize};
use tokio_util::codec::{Decoder, Encoder};
use yykv_types::DsError;

#[derive(Debug, Serialize, Deserialize)]
pub struct D1Request {
    pub sql: String,
    pub params: Vec<JsonValue>,
}

impl D1Request {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let val = oak_json::parse(json)?;
        let obj = val.as_object().ok_or("Expected object")?;

        let sql = obj
            .get("sql")
            .and_then(|v| v.as_str())
            .ok_or("Expected 'sql' field as string")?
            .to_string();

        let params_val = obj
            .get("params")
            .and_then(|v| v.as_array())
            .ok_or("Expected 'params' field as array")?;

        let mut params = Vec::new();
        for p in &params_val.elements {
            params.push(p.clone());
        }

        Ok(D1Request { sql, params })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct D1Response {
    pub success: bool,
    pub results: Option<JsonValue>,
    pub error: Option<String>,
}

impl D1Response {
    pub fn to_json(&self) -> String {
        let mut fields = Vec::new();

        // success
        fields.push(format!("\"success\":{}", self.success));

        // results
        if let Some(ref res) = self.results {
            fields.push(format!("\"results\":{}", res.to_string()));
        } else {
            fields.push("\"results\":null".to_string());
        }

        // error
        if let Some(ref err) = self.error {
            fields.push(format!("\"error\":\"{}\"", err.replace("\"", "\\\"")));
        } else {
            fields.push("\"error\":null".to_string());
        }

        format!("{{{}}}", fields.join(","))
    }
}

pub struct D1Codec;

impl Decoder for D1Codec {
    type Item = D1Request;
    type Error = DsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&src[..4]);
        let len = u32::from_le_bytes(len_bytes) as usize;

        if src.len() < 4 + len {
            src.reserve(len);
            return Ok(None);
        }

        src.advance(4);
        let data = src.split_to(len);
        let json_str = String::from_utf8_lossy(&data);
        let req = D1Request::from_json(&json_str).map_err(DsError::protocol)?;
        Ok(Some(req))
    }
}

impl Encoder<D1Response> for D1Codec {
    type Error = DsError;

    fn encode(&mut self, item: D1Response, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let json = item.to_json();
        let data = json.as_bytes();
        dst.put_u32_le(data.len() as u32);
        dst.put_slice(data);
        Ok(())
    }
}
