// Plik dla traita SequenceEncoder

use crate::error::Result as StemmingResult; 
// use crate::error::StemmingError; // Usunięto, bo nie jest bezpośrednio używane
use crate::dictionary_metadata::EncoderType; 
use std::fmt::Debug; 

/// Trait dla enkoderów/dekoderów sekwencji bajtów.
pub trait SequenceEncoder: Debug + Send + Sync { 
    fn encode<'a>(&self, input: &'a [u8], data: &'a [u8]) -> StemmingResult<Vec<u8>>;
    fn decode<'a>(&self, input: &'a [u8], encoded_data: &'a [u8]) -> StemmingResult<Vec<u8>>;
    fn get_separator(&self) -> Option<u8> {
        None 
    }
    fn get_encoder_type(&self) -> EncoderType;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
