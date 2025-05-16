// Implementacja TrimInfixAndSuffixEncoder

use crate::sequence_encoder_trait::SequenceEncoder;
use crate::error::{Result as StemmingResult, StemmingError};
use crate::dictionary_metadata::EncoderType; // Import EncoderType

#[derive(Debug, Clone)]
pub struct TrimInfixAndSuffixEncoder {
    separator: u8,
}

impl TrimInfixAndSuffixEncoder {
    pub fn new(separator: u8) -> Self {
        TrimInfixAndSuffixEncoder { separator }
    }

    fn common_prefix_len(s1: &[u8], s2: &[u8]) -> usize {
        s1.iter().zip(s2.iter()).take_while(|&(&a, &b)| a == b).count()
    }

    fn common_suffix_len(s1: &[u8], s2: &[u8]) -> usize {
        s1.iter().rev().zip(s2.iter().rev()).take_while(|&(&a, &b)| a == b).count()
    }
}

impl SequenceEncoder for TrimInfixAndSuffixEncoder {
    fn encode<'a>(&self, input: &'a [u8], data: &'a [u8]) -> StemmingResult<Vec<u8>> {
        let sep_pos = data.iter().position(|&b| b == self.separator);

        let (stem_part, tag_part) = match sep_pos {
            Some(pos) => (&data[..pos], &data[pos + 1..]),
            None => (data, Default::default()),
        };

        let p = Self::common_prefix_len(input, stem_part); 

        let input_suffix_part = &input[p..];
        let stem_suffix_part = &stem_part[p..];

        let s = Self::common_suffix_len(input_suffix_part, stem_suffix_part); 

        if input_suffix_part.len() < s {
             return Err(StemmingError::SequenceDecodingError(format!(
                "TrimInfixAndSuffixEncoder: Common suffix length ({}) is greater than input suffix part length ({}) for input '{}', stem '{}'",
                s, input_suffix_part.len(), String::from_utf8_lossy(input), String::from_utf8_lossy(stem_part)
            )));
        }
        let infix_to_remove_from_input_len = input_suffix_part.len() - s;

        if stem_suffix_part.len() < s {
            return Err(StemmingError::SequenceDecodingError(format!(
                "TrimInfixAndSuffixEncoder: Common suffix length ({}) is greater than stem suffix part length ({}) for input '{}', stem '{}'",
                s, stem_suffix_part.len(), String::from_utf8_lossy(input), String::from_utf8_lossy(stem_part)
            )));
        }
        let infix_to_insert_for_stem = &stem_suffix_part[..stem_suffix_part.len() - s];


        if p > 255 || infix_to_remove_from_input_len > 255 {
            return Err(StemmingError::SequenceDecodingError(
                "TrimInfixAndSuffixEncoder: Prefix or infix_to_remove length exceeds 255".to_string(),
            ));
        }

        let mut result = Vec::with_capacity(2 + infix_to_insert_for_stem.len() + tag_part.len());
        result.push(p as u8);
        result.push(infix_to_remove_from_input_len as u8);
        result.extend_from_slice(infix_to_insert_for_stem);
        result.extend_from_slice(tag_part);

        Ok(result)
    }

    fn decode<'a>(&self, input: &'a [u8], encoded_data: &'a [u8]) -> StemmingResult<Vec<u8>> {
        if encoded_data.len() < 2 {
            return Err(StemmingError::SequenceDecodingError(
                "TrimInfixAndSuffixEncoder: Encoded data too short (minimum 2 bytes for lengths).".to_string(),
            ));
        }

        let p_len = encoded_data[0] as usize; 
        let i_len = encoded_data[1] as usize; 

        if p_len > input.len() || p_len.checked_add(i_len).map_or(true, |sum| sum > input.len()) {
             return Err(StemmingError::SequenceDecodingError(format!(
                "TrimInfixAndSuffixEncoder: Invalid lengths p ({}) or i ({}) for input length {} ('{}')",
                p_len, i_len, input.len(), String::from_utf8_lossy(input)
            )));
        }

        let prefix_from_input = &input[..p_len];
        let suffix_from_input = &input[p_len + i_len ..];
        let stem_infix_and_tag_from_fsa = &encoded_data[2..];

        let mut result = Vec::with_capacity(
            prefix_from_input.len() + stem_infix_and_tag_from_fsa.len() + suffix_from_input.len()
        );
        result.extend_from_slice(prefix_from_input);
        result.extend_from_slice(stem_infix_and_tag_from_fsa);
        result.extend_from_slice(suffix_from_input);
        
        Ok(result)
    }

    fn get_separator(&self) -> Option<u8> {
        Some(self.separator)
    }

    fn get_encoder_type(&self) -> EncoderType {
        EncoderType::Infix // Zakładamy, że ten enkoder odpowiada typowi Infix
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_infix_suffix_encoder_basic() {
        let encoder = TrimInfixAndSuffixEncoder::new(b'+');
        let input1 = b"niebieski"; 
        let data1 = b"niesk+ADJ";   
        let encoded1 = encoder.encode(input1, data1).unwrap();
        assert_eq!(encoded1, vec![1, 4, b'A', b'D', b'J']);

        let decoded1 = encoder.decode(input1, &encoded1).unwrap();
        assert_eq!(decoded1, b"nADJeski");

        let input2 = b"najpiekniejszy";
        let data2 = b"piekny+ADJ";
        let encoded2 = encoder.encode(input2, data2).unwrap();
        assert_eq!(encoded2, vec![0, 15, b'p',b'i',b'e',b'k',b'n', b'A',b'D',b'J']);
        
        let decoded2 = encoder.decode(input2, &encoded2).unwrap();
        assert_eq!(decoded2, b"pieknADJy"); 
    }


    #[test]
    fn test_no_infix_no_suffix_just_prefix() {
        let encoder = TrimInfixAndSuffixEncoder::new(b'+');
        let input = b"przedrostek";
        let data = b"rostek+X";
        let encoded = encoder.encode(input, data).unwrap();
        assert_eq!(encoded, vec![0, 5, b'X']);

        let decoded = encoder.decode(input, &encoded).unwrap();
        assert_eq!(decoded, b"Xrostek"); 
    }

    #[test]
    fn test_decode_encoded_too_short() {
        let encoder = TrimInfixAndSuffixEncoder::new(b'+');
        let input = b"word";
        let encoded_data_short = &[1];
        let result = encoder.decode(input, encoded_data_short);
        assert!(matches!(result, Err(StemmingError::SequenceDecodingError(_))));
    }

    #[test]
    fn test_decode_invalid_lengths() {
        let encoder = TrimInfixAndSuffixEncoder::new(b'+');
        let input = b"word"; 
        let encoded_data = &[3, 2, b'T']; 
        let result = encoder.decode(input, encoded_data);
        assert!(matches!(result, Err(StemmingError::SequenceDecodingError(_))));
    }

    #[test]
    fn test_trim_infix_suffix_encoder_get_type() {
        let encoder = TrimInfixAndSuffixEncoder::new(b'+');
        assert_eq!(encoder.get_encoder_type(), EncoderType::Infix);
    }
}
