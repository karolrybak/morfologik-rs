// Definicja struktury PolishStemmer i jej implementacje

use std::path::{Path, PathBuf};
use std::sync::Arc;

use morfologik_stemming::dictionary::Dictionary;
use morfologik_stemming::dictionary_lookup::DictionaryLookup;
use morfologik_stemming::dictionary_metadata::DictionaryMetadata;
use morfologik_stemming::stemmer_trait::Stemmer;
use morfologik_stemming::word_data::WordData;
use morfologik_stemming::error::{Result as StemmingResult, StemmingError};

// Upewnij się, że te ścieżki są poprawne względem lokalizacji tego pliku (polish_stemmer.rs)
// Jeśli polish_stemmer.rs jest w morfologik-rs/crates/morfologik-polish/src/
// a zasoby w morfologik-rs/crates/morfologik-polish/resources/
// to ścieżka powinna być "../resources/..."
const EMBEDDED_POLISH_DICT_BYTES: &'static [u8] = 
    include_bytes!("../resources/morfologik/stemming/polish/polish.dict");
const EMBEDDED_POLISH_INFO_BYTES: &'static [u8] = 
    include_bytes!("../resources/morfologik/stemming/polish/polish.info");


/// Stemer dla języka polskiego.
#[derive(Debug, Clone)]
pub struct PolishStemmer {
    dictionary_lookup: DictionaryLookup,
}

impl PolishStemmer {
    pub fn new() -> StemmingResult<Self> {
        println!("[PolishStemmer::new()] Próba wczytania osadzonego słownika.");
        println!("[PolishStemmer::new()] Długość EMBEDDED_POLISH_DICT_BYTES: {}", EMBEDDED_POLISH_DICT_BYTES.len());
        println!("[PolishStemmer::new()] Długość EMBEDDED_POLISH_INFO_BYTES: {}", EMBEDDED_POLISH_INFO_BYTES.len());

        if EMBEDDED_POLISH_DICT_BYTES.is_empty() {
            eprintln!("[PolishStemmer::new()] BŁĄD: Osadzony plik .dict jest pusty!");
            return Err(StemmingError::DictionaryConfigurationError("Osadzony plik .dict jest pusty.".to_string()));
        }
        if EMBEDDED_POLISH_INFO_BYTES.is_empty() {
            eprintln!("[PolishStemmer::new()] BŁĄD: Osadzony plik .info jest pusty!");
            return Err(StemmingError::DictionaryConfigurationError("Osadzony plik .info jest pusty.".to_string()));
        }

        // Wyświetl kilka pierwszych bajtów dla weryfikacji
        let dict_preview_len = std::cmp::min(16, EMBEDDED_POLISH_DICT_BYTES.len());
        println!("[PolishStemmer::new()] Początek .dict (max 16B): {:?}", &EMBEDDED_POLISH_DICT_BYTES[..dict_preview_len]);
        let info_preview_len = std::cmp::min(64, EMBEDDED_POLISH_INFO_BYTES.len());
        println!("[PolishStemmer::new()] Początek .info (max 64B): {:?}", String::from_utf8_lossy(&EMBEDDED_POLISH_INFO_BYTES[..info_preview_len]));


        let dictionary = Dictionary::from_embedded(
            EMBEDDED_POLISH_DICT_BYTES,
            EMBEDDED_POLISH_INFO_BYTES,
        )?;
        println!("[PolishStemmer::new()] Dictionary::from_embedded zakończone sukcesem.");
        let dictionary_lookup = DictionaryLookup::new(Arc::new(dictionary));
        Ok(PolishStemmer {
            dictionary_lookup,
        })
    }

    pub fn from_path<P: AsRef<Path>>(dictionary_file_path: P) -> StemmingResult<Self> {
        let path_ref = dictionary_file_path.as_ref();
        
        if !path_ref.exists() {
            return Err(StemmingError::Io(format!(
                "Plik słownika polskiego nie został znaleziony: {}",
                path_ref.display()
            )));
        }
        
        let info_path = Dictionary::get_info_file_path(path_ref);
        if !info_path.exists() {
             return Err(StemmingError::MetadataNotFound(
                info_path.to_string_lossy().into_owned(),
            ));
        }

        let dictionary = Dictionary::from_file(path_ref)?;
        let dictionary_lookup = DictionaryLookup::new(Arc::new(dictionary));
        Ok(PolishStemmer {
            dictionary_lookup,
        })
    }
}

impl Stemmer for PolishStemmer {
    fn lookup(&self, word: &[u8]) -> StemmingResult<Vec<WordData>> {
        self.dictionary_lookup.lookup(word)
    }

    fn get_dictionary_metadata(&self) -> &DictionaryMetadata {
        self.dictionary_lookup.get_dictionary_metadata()
    }
}
