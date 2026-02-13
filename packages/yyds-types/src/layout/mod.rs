use yykv_types::layout::{DsValueEncoder, VALUE_MAGIC};
pub use yykv_types::layout::{ValueHeader, ValueTypeId};

/// YYDS Binary Decoder for YYValue.
pub type YYValueDecoder = yykv_types::layout::YYValueDecoder;

/// YYDS Binary Encoder for YYValue.
pub struct YYDSValueEncoder;

impl YYDSValueEncoder {
    /// Encode a YYValue into its binary representation.
    pub fn encode(value: &crate::YYValue) -> Result<bytes::Bytes, crate::YYError> {
        DsValueEncoder::encode(value)
    }

    /// Get the magic bytes used for YYDS binary format.
    pub fn magic() -> [u8; 2] {
        VALUE_MAGIC
    }
}
