// Implementacja NoEncoder

use crate::sequence_encoder_trait::SequenceEncoder;
use crate::error::Result as StemmingResult;
use crate::dictionary_metadata::EncoderType; // Import EncoderType

#[derive(Debug, Default, Clone)]
pub struct NoEncoder;

impl NoEncoder {
    pub fn new() -> Self {
        NoEncoder
    }
}

impl SequenceEncoder for NoEncoder {
    fn encode<'a>(&self, _input: &'a [u8], data: &'a [u8]) -> StemmingResult<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn decode<'a>(&self, _input: &'a [u8], encoded_data: &'a [u8]) -> StemmingResult<Vec<u8>> {
        Ok(encoded_data.to_vec())
    }

    fn get_encoder_type(&self) -> EncoderType {
        EncoderType::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_encoder_encode() {
        let encoder = NoEncoder::new();
        let input = b"testword";
        let data = b"stem+tag";
        let encoded = encoder.encode(input, data).unwrap();
        assert_eq!(encoded, data);
    }

    #[test]
    fn test_no_encoder_decode() {
        let encoder = NoEncoder::new();
        let input = b"testword"; 
        let encoded_data = b"stem+tag";
        let decoded = encoder.decode(input, encoded_data).unwrap();
        assert_eq!(decoded, encoded_data);
    }

    #[test]
    fn test_no_encoder_get_type() {
        let encoder = NoEncoder::new();
        assert_eq!(encoder.get_encoder_type(), EncoderType::None);
    }
}
