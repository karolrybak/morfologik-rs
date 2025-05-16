// Implementacja TrimPrefixAndSuffixEncoder

use crate::sequence_encoder_trait::SequenceEncoder;
use crate::error::{Result as StemmingResult, StemmingError};
use crate::dictionary_metadata::EncoderType; // Import EncoderType

#[derive(Debug, Default, Clone)]
pub struct TrimPrefixAndSuffixEncoder {
    separator: u8,
}

impl TrimPrefixAndSuffixEncoder {
    pub fn new(separator: u8) -> Self {
        TrimPrefixAndSuffixEncoder { separator }
    }

    fn common_prefix_len(s1: &[u8], s2: &[u8]) -> usize {
        s1.iter().zip(s2.iter()).take_while(|&(&a, &b)| a == b).count()
    }

    fn common_suffix_len(s1: &[u8], s2: &[u8]) -> usize {
        s1.iter().rev().zip(s2.iter().rev()).take_while(|&(&a, &b)| a == b).count()
    }
}

impl SequenceEncoder for TrimPrefixAndSuffixEncoder {
    fn encode<'a>(&self, input: &'a [u8], data: &'a [u8]) -> StemmingResult<Vec<u8>> {
        let sep_pos = data.iter().position(|&b| b == self.separator);

        let (stem_part, tag_part) = match sep_pos {
            Some(pos) => (&data[..pos], &data[pos + 1..]),
            None => (data, Default::default()), 
        };

        let p = Self::common_prefix_len(input, stem_part);
        let s = Self::common_suffix_len(input, stem_part);

        if p + s > input.len() || p + s > stem_part.len() {
             return Err(StemmingError::SequenceDecodingError(format!(
                "TrimPrefixAndSuffixEncoder: Prefix and suffix overlap for input '{}' and stem '{}' (p={}, s={})",
                String::from_utf8_lossy(input), String::from_utf8_lossy(stem_part), p, s
            )));
        }

        let input_core = &input[p .. input.len() - s];
        let stem_part_core = &stem_part[p .. stem_part.len() - s];

        if input_core != stem_part_core {
            return Err(StemmingError::SequenceDecodingError(format!(
                "TrimPrefixAndSuffixEncoder: Core mismatch for input '{}' (core '{}') and stem '{}' (core '{}')",
                String::from_utf8_lossy(input), String::from_utf8_lossy(input_core),
                String::from_utf8_lossy(stem_part), String::from_utf8_lossy(stem_part_core)
            )));
        }

        let bytes_prefix_to_remove = p;
        let bytes_suffix_to_remove = s;


        if bytes_prefix_to_remove > 255 || bytes_suffix_to_remove > 255 {
            return Err(StemmingError::SequenceDecodingError(
                "TrimPrefixAndSuffixEncoder: Too many bytes for prefix/suffix to remove (max 255)".to_string(),
            ));
        }

        let mut result = Vec::with_capacity(2 + tag_part.len());
        result.push(bytes_prefix_to_remove as u8);
        result.push(bytes_suffix_to_remove as u8);
        result.extend_from_slice(tag_part);

        Ok(result)
    }

    fn decode<'a>(&self, input: &'a [u8], encoded_data: &'a [u8]) -> StemmingResult<Vec<u8>> {
        if encoded_data.len() < 2 {
            return Err(StemmingError::SequenceDecodingError(
                "TrimPrefixAndSuffixEncoder: Encoded data is too short (minimum 2 bytes for lengths).".to_string(),
            ));
        }

        let p = encoded_data[0] as usize; 
        let s = encoded_data[1] as usize; 

        if p + s > input.len() {
            return Err(StemmingError::SequenceDecodingError(format!(
                "TrimPrefixAndSuffixEncoder: Prefix (len {}) + suffix (len {}) to remove is greater than input length {} ('{}')",
                p, s, input.len(), String::from_utf8_lossy(input)
            )));
        }

        let stem_part = &input[p .. input.len() - s];
        let tag_part_from_fsa = &encoded_data[2..];

        let mut result = Vec::with_capacity(stem_part.len() + tag_part_from_fsa.len());
        result.extend_from_slice(stem_part);
        result.extend_from_slice(tag_part_from_fsa);
        
        Ok(result)
    }

    fn get_separator(&self) -> Option<u8> {
        Some(self.separator)
    }
    
    fn get_encoder_type(&self) -> EncoderType {
        // W Morfologiku Java, InfixEncoder często jest aliasem lub implementuje to samo co TrimPrefixAndSuffix.
        // Jeśli mamy osobny EncoderType::Infix, to ten powinien zwracać coś innego,
        // np. EncoderType::PrefixSuffix. Na razie zakładamy, że Infix to ten enkoder.
        EncoderType::Infix // Lub dedykowany typ, jeśli dodamy do EncoderType
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_prefix_suffix_encoder_basic() {
        let encoder = TrimPrefixAndSuffixEncoder::new(b'+');
        let input1 = b"niebieski";
        let data1 = b"biesk+ADJ";
        let encoded1 = encoder.encode(input1, data1).unwrap();
        assert_eq!(encoded1, vec![3, 1, b'A', b'D', b'J']);

        let decoded1 = encoder.decode(input1, &encoded1).unwrap();
        assert_eq!(decoded1, b"bieskADJ");

        let input2 = b"kot";
        let data2 = b"kot+N";
        let encoded2 = encoder.encode(input2, data2).unwrap();
        assert_eq!(encoded2, vec![0, 0, b'N']);
        let decoded2 = encoder.decode(input2, &encoded2).unwrap();
        assert_eq!(decoded2, b"kotN");

        let input3 = b"przedrostek";
        let data3 = b"rostek+X";
        let encoded3 = encoder.encode(input3, data3).unwrap();
        assert_eq!(encoded3, vec![5, 0, b'X']);
        let decoded3 = encoder.decode(input3, &encoded3).unwrap();
        assert_eq!(decoded3, b"rostekX");

        let input4 = b"rosteksufiks";
        let data4 = b"rostek+Y";
        let encoded4 = encoder.encode(input4, data4).unwrap();
        assert_eq!(encoded4, vec![0, 6, b'Y']);
        let decoded4 = encoder.decode(input4, &encoded4).unwrap();
        assert_eq!(decoded4, b"rostekY");
    }

    #[test]
    fn test_trim_prefix_suffix_encoder_no_tag() {
        let encoder = TrimPrefixAndSuffixEncoder::new(b'+');
        let input = b"niebieski";
        let data_no_tag = b"biesk";
        let encoded = encoder.encode(input, data_no_tag).unwrap();
        assert_eq!(encoded, vec![3, 1]);

        let decoded = encoder.decode(input, &encoded).unwrap();
        assert_eq!(decoded, b"biesk");
    }
    
    #[test]
    fn test_encode_core_mismatch() {
        let encoder = TrimPrefixAndSuffixEncoder::new(b'+');
        let input = b"abXYcd";
        let data = b"abZZcd+T";
        let result = encoder.encode(input, data);
        assert!(matches!(result, Err(StemmingError::SequenceDecodingError(_))));
    }

    #[test]
    fn test_encode_overlap() {
        let encoder = TrimPrefixAndSuffixEncoder::new(b'+');
        let input = b"abc";
        let data = b"axc+T";
        let result = encoder.encode(input, data);
        assert!(matches!(result, Err(StemmingError::SequenceDecodingError(_))));
    }

    #[test]
    fn test_decode_encoded_too_short() {
        let encoder = TrimPrefixAndSuffixEncoder::new(b'+');
        let input = b"word";
        let encoded_data_short = &[1]; 
        let result = encoder.decode(input, encoded_data_short);
        assert!(matches!(result, Err(StemmingError::SequenceDecodingError(_))));
    }

    #[test]
    fn test_decode_ps_greater_than_input() {
        let encoder = TrimPrefixAndSuffixEncoder::new(b'+');
        let input = b"word"; 
        let encoded_data = &[3, 2, b'T']; 
        let result = encoder.decode(input, encoded_data);
        assert!(matches!(result, Err(StemmingError::SequenceDecodingError(_))));
    }

    #[test]
    fn test_trim_prefix_suffix_encoder_get_type() {
        let encoder = TrimPrefixAndSuffixEncoder::new(b'+');
        assert_eq!(encoder.get_encoder_type(), EncoderType::Infix); // Zakładając, że Infix to ten
    }
}
