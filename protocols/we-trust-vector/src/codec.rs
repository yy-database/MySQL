use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use yykv_types::DsError;

#[derive(Debug, Clone)]
pub struct VectorSearchRequest {
    pub collection: String,
    pub vector: Vec<f32>,
    pub top_k: u32,
}

#[derive(Debug, Clone)]
pub struct VectorSearchResponse {
    pub ids: Vec<u64>,
    pub scores: Vec<f32>,
}

pub struct VectorCodec;

impl Decoder for VectorCodec {
    type Item = VectorSearchRequest;
    type Error = DsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 8 {
            return Ok(None);
        }

        let name_len = u32::from_le_bytes([src[0], src[1], src[2], src[3]]) as usize;
        let vec_dim = u32::from_le_bytes([src[4], src[5], src[6], src[7]]) as usize;
        let total_len = 8 + name_len + vec_dim * 4 + 4;

        if src.len() < total_len {
            src.reserve(total_len);
            return Ok(None);
        }

        src.advance(8);
        let collection = String::from_utf8_lossy(&src.split_to(name_len)).to_string();

        let mut vector = Vec::with_capacity(vec_dim);
        for _ in 0..vec_dim {
            vector.push(src.get_f32_le());
        }

        let top_k = src.get_u32_le();

        Ok(Some(VectorSearchRequest {
            collection,
            vector,
            top_k,
        }))
    }
}

impl Encoder<VectorSearchResponse> for VectorCodec {
    type Error = DsError;

    fn encode(
        &mut self,
        item: VectorSearchResponse,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        dst.put_u32_le(item.ids.len() as u32);
        for id in item.ids {
            dst.put_u64_le(id);
        }
        for score in item.scores {
            dst.put_f32_le(score);
        }
        Ok(())
    }
}
