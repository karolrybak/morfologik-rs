// Główny plik biblioteki morfologik-stemming
pub mod dictionary_metadata;
pub mod error;
pub mod word_data;
pub mod stemmer_trait;
pub mod sequence_encoder_trait;
pub mod encoders;
pub mod dictionary;
pub mod dictionary_lookup; // Dodajemy nowy moduł

// Przykładowa funkcja, aby crate się kompilował
pub fn stemming_add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = stemming_add(2, 2);
        assert_eq!(result, 4);
    }
}
