// Plik dla struktury DictionaryLookup

use std::sync::Arc;

// morfologik_fsa::fsa_trait::Fsa jest używane w typie Box<dyn Fsa...>
use morfologik_fsa::iterator::ByteSequenceIterator; 

use crate::dictionary::Dictionary;
use crate::word_data::WordData;
use crate::stemmer_trait::Stemmer;
use crate::error::{Result as StemmingResult, StemmingError}; 
use crate::dictionary_metadata::DictionaryMetadata; 


/// Struktura odpowiedzialna za wyszukiwanie form podstawowych i tagów w słowniku.
#[derive(Debug, Clone)] 
pub struct DictionaryLookup {
    dictionary: Arc<Dictionary>,
}

impl DictionaryLookup {
    pub fn new(dictionary: Arc<Dictionary>) -> Self {
        DictionaryLookup { dictionary }
    }

    fn split_decoded_data_to_stem_and_tag<'a>(
        &self,
        decoded_data: &'a [u8],
        separator: u8,
    ) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        if let Some(sep_pos) = decoded_data.iter().position(|&b| b == separator) {
            let stem_part = &decoded_data[..sep_pos];
            let tag_part = &decoded_data[sep_pos + 1..];

            let stem = if stem_part.is_empty() { None } else { Some(stem_part.to_vec()) };
            let tag = if tag_part.is_empty() { None } else { Some(tag_part.to_vec()) };
            (stem, tag)
        } else {
            (Some(decoded_data.to_vec()).filter(|s| !s.is_empty()), None)
        }
    }
}

impl Stemmer for DictionaryLookup {
    fn lookup(&self, word: &[u8]) -> StemmingResult<Vec<WordData>> {
        let mut forms: Vec<WordData> = Vec::new();

        let fsa = self.dictionary.fsa.as_ref().as_ref(); 
        let encoder = self.dictionary.encoder.as_ref().as_ref(); 
        let metadata = self.dictionary.metadata.as_ref(); 

        let mut current_node = fsa.get_root_node();
        let mut word_path_exists = true;
        for &byte in word.iter() {
            match fsa.get_arc(current_node, byte) {
                Ok(arc) => {
                    match fsa.get_end_node(arc) {
                        Ok(next_node) => current_node = next_node,
                        Err(_) => { 
                            word_path_exists = false;
                            break;
                        }
                    }
                }
                Err(_) => { 
                    word_path_exists = false;
                    break;
                }
            }
        }

        if !word_path_exists {
            return Ok(forms); 
        }

        let value_iterator = ByteSequenceIterator::from_node(fsa, current_node);
        let separator = metadata.get_separator()?; 

        for encoded_data_result in value_iterator {
            let encoded_data_from_fsa = encoded_data_result.map_err(StemmingError::Fsa)?;
            
            let decoded_stem_plus_tag = encoder.decode(word, &encoded_data_from_fsa)?;

            let (stem_bytes, tag_bytes) =
                self.split_decoded_data_to_stem_and_tag(&decoded_stem_plus_tag, separator as u8);

            forms.push(WordData::new(
                word.to_vec(), 
                stem_bytes,    
                tag_bytes,     
            ));
        }

        Ok(forms)
    }

    fn get_dictionary_metadata(&self) -> &DictionaryMetadata {
        &self.dictionary.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::Dictionary;
    use crate::dictionary_metadata::{DictionaryMetadata, EncoderType};
    use crate::encoders::NoEncoder; 
    use morfologik_fsa::fsa_trait::Fsa; // Potrzebne dla Box<dyn Fsa>
    use morfologik_fsa::fsa5::FSA5; 
    use morfologik_fsa::fsa_header::{FSA_MAGIC, VERSION_FSA5, FsaFlags};
    use std::io::Cursor; 

    fn create_test_fsa_for_lookup(key: &[u8], value_as_single_sequence: &[u8]) -> FSA5 {
        let mut arcs_data = Vec::new();
        let combined_sequence: Vec<u8> = key.iter().chain(value_as_single_sequence.iter()).cloned().collect();
        let total_len = combined_sequence.len();

        for (i, &byte_val) in combined_sequence.iter().enumerate() {
            arcs_data.push(byte_val); 
            let mut arc_flags = 0u8;
            if i == total_len - 1 { 
                arc_flags |= morfologik_fsa::fsa5::BIT_ARC_FINAL;
            }
            if i == total_len - 1 { 
                 arc_flags |= morfologik_fsa::fsa5::BIT_ARC_LAST;
            }
            arcs_data.push(arc_flags);
            arcs_data.push(if i < total_len - 1 { ((i + 1) * 3) as u8 } else { 0 }); 
        }
        
        let mut header_data = Vec::new();
        header_data.extend_from_slice(&FSA_MAGIC);
        header_data.push(VERSION_FSA5);
        header_data.push(1); 
        header_data.push(1); 
        header_data.extend_from_slice(&FsaFlags::empty().bits().to_le_bytes()); 

        let mut fsa_file_bytes = header_data;
        fsa_file_bytes.extend_from_slice(&arcs_data);
        
        FSA5::from_reader(&mut Cursor::new(fsa_file_bytes)).unwrap()
    }

    #[test]
    fn test_dictionary_lookup_simple_no_encoder() {
        let key = b"kot";
        let stem_val = b"KOT";
        let tag_val = b"SUBST";
        let separator_char = b'+';

        let mut value_to_encode = stem_val.to_vec();
        value_to_encode.push(separator_char);
        value_to_encode.extend_from_slice(tag_val); 

        let fsa_impl = create_test_fsa_for_lookup(key, &value_to_encode);
        let fsa: Arc<Box<dyn Fsa + Send + Sync>> = Arc::new(Box::new(fsa_impl));


        let mut metadata = DictionaryMetadata::new();
        metadata.set_attribute("fsa.dict.separator".to_string(), (separator_char as char).to_string());
        metadata.set_attribute("fsa.dict.encoder".to_string(), EncoderType::None.as_str().to_string());
        let arc_metadata = Arc::new(metadata);

        let encoder: Arc<Box<dyn crate::sequence_encoder_trait::SequenceEncoder + Send + Sync>> 
            = Arc::new(Box::new(NoEncoder::new()));
        
        let dictionary = Arc::new(Dictionary {
            fsa,
            metadata: arc_metadata,
            encoder,
        });
        let lookup_service = DictionaryLookup::new(dictionary);

        let results = lookup_service.lookup(key).expect("Lookup failed");

        assert_eq!(results.len(), 1, "Expected one result");
        let word_data = &results[0];

        assert_eq!(word_data.word(), key, "Word mismatch");
        assert_eq!(word_data.stem().unwrap_or_default(), stem_val, "Stem mismatch");
        assert_eq!(word_data.tag().unwrap_or_default(), tag_val, "Tag mismatch");
    }

     #[test]
    fn test_lookup_word_not_in_fsa() {
        let key = b"jest";
        let value = b"BYC+VERB";
        let fsa_impl = create_test_fsa_for_lookup(key, value);
        let fsa: Arc<Box<dyn Fsa + Send + Sync>> = Arc::new(Box::new(fsa_impl));
        
        let mut metadata = DictionaryMetadata::new();
        metadata.set_attribute("fsa.dict.separator".to_string(), "+".to_string());
        metadata.set_attribute("fsa.dict.encoder".to_string(), EncoderType::None.as_str().to_string());
        let arc_metadata = Arc::new(metadata);
        
        let encoder: Arc<Box<dyn crate::sequence_encoder_trait::SequenceEncoder + Send + Sync>> 
            = Arc::new(Box::new(NoEncoder::new()));
        
        let dictionary = Arc::new(Dictionary { fsa, metadata: arc_metadata, encoder });
        let lookup_service = DictionaryLookup::new(dictionary);

        let results = lookup_service.lookup(b"nieistnieje").expect("Lookup failed but shouldn't panic");
        assert!(results.is_empty(), "Expected no results for a non-existent word");
    }

    #[test]
    fn test_lookup_empty_word() {
        let key = b"a"; 
        let value = b"A+X";
        let fsa_impl = create_test_fsa_for_lookup(key, value);
        let fsa: Arc<Box<dyn Fsa + Send + Sync>> = Arc::new(Box::new(fsa_impl));
        
        let mut metadata = DictionaryMetadata::new();
        metadata.set_attribute("fsa.dict.separator".to_string(), "+".to_string());
        metadata.set_attribute("fsa.dict.encoder".to_string(), EncoderType::None.as_str().to_string());
        let arc_metadata = Arc::new(metadata);
        
        let encoder: Arc<Box<dyn crate::sequence_encoder_trait::SequenceEncoder + Send + Sync>>
            = Arc::new(Box::new(NoEncoder::new()));
        
        let dictionary = Arc::new(Dictionary { fsa, metadata: arc_metadata, encoder });
        let lookup_service = DictionaryLookup::new(dictionary);

        let results = lookup_service.lookup(b"").expect("Lookup for empty word failed");
        assert!(results.is_empty(), "Expected no results for an empty word if not in FSA");
    }
}
