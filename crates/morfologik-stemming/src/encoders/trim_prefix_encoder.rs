// Implementacja TrimPrefixEncoder

use crate::sequence_encoder_trait::SequenceEncoder;
use crate::error::{Result as StemmingResult, StemmingError};
use crate::dictionary_metadata::EncoderType; // Import EncoderType

#[derive(Debug, Default, Clone)]
pub struct TrimPrefixEncoder {
    separator: u8,
}

impl TrimPrefixEncoder {
    pub fn new(separator: u8) -> Self {
        TrimPrefixEncoder { separator }
    }

    fn common_suffix_len(s1: &[u8], s2: &[u8]) -> usize {
        s1.iter().rev().zip(s2.iter().rev()).take_while(|&(&a, &b)| a == b).count()
    }
}

impl SequenceEncoder for TrimPrefixEncoder {
    fn encode<'a>(&self, input: &'a [u8], data: &'a [u8]) -> StemmingResult<Vec<u8>> {
        let sep_pos = data.iter().position(|&b| b == self.separator);

        let (stem_part, tag_part) = match sep_pos {
            Some(pos) => (&data[..pos], &data[pos + 1..]),
            None => (data, Default::default()), 
        };

        let suffix_len = Self::common_suffix_len(input, stem_part);

        if suffix_len == 0 && !input.is_empty() && !stem_part.is_empty() && input != stem_part {
            return Err(StemmingError::SequenceDecodingError(format!(
                "TrimPrefixEncoder: No common suffix between input ('{}') and stem ('{}')",
                String::from_utf8_lossy(input), String::from_utf8_lossy(stem_part)
            )));
        }
        
        let bytes_to_remove_from_input_prefix = input.len() - suffix_len;

        if bytes_to_remove_from_input_prefix > 255 {
            return Err(StemmingError::SequenceDecodingError(
                "TrimPrefixEncoder: Too many bytes to remove from input prefix (max 255)".to_string(),
            ));
        }

        let mut result = Vec::with_capacity(1 + tag_part.len());
        result.push(bytes_to_remove_from_input_prefix as u8);
        result.extend_from_slice(tag_part);

        Ok(result)
    }

    fn decode<'a>(&self, input: &'a [u8], encoded_data: &'a [u8]) -> StemmingResult<Vec<u8>> {
        if encoded_data.is_empty() {
            return Err(StemmingError::SequenceDecodingError(
                "TrimPrefixEncoder: Encoded data is empty, cannot decode.".to_string(),
            ));
        }

        let bytes_to_remove_from_input_prefix = encoded_data[0] as usize;

        if bytes_to_remove_from_input_prefix > input.len() {
            return Err(StemmingError::SequenceDecodingError(format!(
                "TrimPrefixEncoder: Cannot remove {} bytes from input prefix of length {} ('{}')",
                bytes_to_remove_from_input_prefix, input.len(), String::from_utf8_lossy(input)
            )));
        }

        let stem_part = &input[bytes_to_remove_from_input_prefix..];
        let tag_part_from_fsa = &encoded_data[1..];

        let mut result = Vec::with_capacity(stem_part.len() + tag_part_from_fsa.len());
        result.extend_from_slice(stem_part);
        result.extend_from_slice(tag_part_from_fsa);
        
        Ok(result)
    }

    fn get_separator(&self) -> Option<u8> {
        Some(self.separator)
    }

    fn get_encoder_type(&self) -> EncoderType {
        EncoderType::Prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_prefix_encoder_basic() {
        let encoder = TrimPrefixEncoder::new(b'+');
        let input1 = b"przedimek";
        let data1 = b"dimek+N";
        let encoded1 = encoder.encode(input1, data1).unwrap();
        assert_eq!(encoded1, vec![3, b'N']);

        let decoded1 = encoder.decode(input1, &encoded1).unwrap();
        assert_eq!(decoded1, b"dimekN");

        let input2 = b"kotami";
        let data2 = b"ami+X";
        let encoded2 = encoder.encode(input2, data2).unwrap();
        assert_eq!(encoded2, vec![3, b'X']);

        let decoded2 = encoder.decode(input2, &encoded2).unwrap();
        assert_eq!(decoded2, b"amiX");
    }

    #[test]
    fn test_trim_prefix_encoder_no_tag() {
        let encoder = TrimPrefixEncoder::new(b'+');
        let input = b"przeddom";
        let data_no_tag = b"dom";
        let encoded = encoder.encode(input, data_no_tag).unwrap();
        assert_eq!(encoded, vec![5]);

        let decoded = encoder.decode(input, &encoded).unwrap();
        assert_eq!(decoded, b"dom");
    }

    #[test]
    fn test_trim_prefix_encoder_stem_equals_word_no_tag() {
        let encoder = TrimPrefixEncoder::new(b'+');
        let input = b"kot";
        let data_stem_equals_word = b"kot";
        let encoded = encoder.encode(input, data_stem_equals_word).unwrap();
        assert_eq!(encoded, vec![0]);

        let decoded = encoder.decode(input, &encoded).unwrap();
        assert_eq!(decoded, b"kot");
    }

    #[test]
    fn test_trim_prefix_encoder_stem_equals_word_with_tag() {
        let encoder = TrimPrefixEncoder::new(b'+');
        let input = b"kot";
        let data = b"kot+N";
        let encoded = encoder.encode(input, data).unwrap();
        assert_eq!(encoded, vec![0, b'N']);

        let decoded = encoder.decode(input, &encoded).unwrap();
        assert_eq!(decoded, b"kotN");
    }
    
    #[test]
    fn test_trim_prefix_encoder_decode_empty_encoded_data() {
        let encoder = TrimPrefixEncoder::new(b'+');
        let input = b"word";
        let encoded_data = b"";
        let result = encoder.decode(input, encoded_data);
        assert!(matches!(result, Err(StemmingError::SequenceDecodingError(_))));
    }

    #[test]
    fn test_trim_prefix_encoder_decode_bytes_to_remove_too_large() {
        let encoder = TrimPrefixEncoder::new(b'+');
        let input = b"word"; 
        let encoded_data = &[5, b't', b'a', b'g']; 
        let result = encoder.decode(input, encoded_data);
        assert!(matches!(result, Err(StemmingError::SequenceDecodingError(_))));
    }

    #[test]
    fn test_trim_prefix_encoder_no_common_suffix() {
        let encoder = TrimPrefixEncoder::new(b'+');
        let input = b"abcdef";
        let data = b"xyz+tag"; 
        let result = encoder.encode(input, data);
        assert!(matches!(result, Err(StemmingError::SequenceDecodingError(_))));
    }

    #[test]
    fn test_trim_prefix_encoder_get_type() {
        let encoder = TrimPrefixEncoder::new(b'+');
        assert_eq!(encoder.get_encoder_type(), EncoderType::Prefix);
    }
}
