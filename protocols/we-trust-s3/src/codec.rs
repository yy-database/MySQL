use bytes::{BufMut, BytesMut};
use http::{Request, Response};
use tokio_util::codec::{Decoder, Encoder};
use yykv_types::DsError;

pub struct S3HttpCodec;

impl Decoder for S3HttpCodec {
    type Item = Request<Vec<u8>>;
    type Error = DsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 非常简化的 HTTP 请求解析，仅用于演示 Framed 模式
        let header_end = src.windows(4).position(|w| w == b"\r\n\r\n");
        if let Some(pos) = header_end {
            let header_part = src.split_to(pos + 4);
            let header_str = String::from_utf8_lossy(&header_part);

            // 解析 Content-Length
            let mut content_len = 0;
            for line in header_str.lines() {
                if line.to_lowercase().starts_with("content-length:") {
                    content_len = line["content-length:".len()..].trim().parse().unwrap_or(0);
                }
            }

            if src.len() < content_len {
                return Ok(None);
            }

            let body = src.split_to(content_len).to_vec();

            // 构建 Request (简化实现)
            let req = Request::builder()
                .method("GET")
                .uri("/")
                .body(body)
                .map_err(|e| DsError::protocol(e.to_string()))?;

            Ok(Some(req))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<Response<Vec<u8>>> for S3HttpCodec {
    type Error = DsError;

    fn encode(&mut self, item: Response<Vec<u8>>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let status = item.status();
        let body = item.into_body();

        let response_str = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n\r\n",
            status.as_u16(),
            status.canonical_reason().unwrap_or(""),
            body.len()
        );

        dst.put_slice(response_str.as_bytes());
        dst.put_slice(&body);
        Ok(())
    }
}
