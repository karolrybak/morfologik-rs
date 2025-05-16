// Definicja struktury PolishStemmer i jej implementacje

use std::path::Path; // PathBuf nie jest tu używane bezpośrednio
use std::sync::Arc;


// Importy z crate morfologik-stemming
use morfologik_stemming::dictionary::Dictionary;
use morfologik_stemming::dictionary_lookup::DictionaryLookup;
use morfologik_stemming::dictionary_metadata::DictionaryMetadata;
use morfologik_stemming::stemmer_trait::Stemmer;
use morfologik_stemming::word_data::WordData;
use morfologik_stemming::error::{Result as StemmingResult, StemmingError};

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
        let dictionary = Dictionary::from_embedded(
            EMBEDDED_POLISH_DICT_BYTES,
            EMBEDDED_POLISH_INFO_BYTES,
        )?;
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
