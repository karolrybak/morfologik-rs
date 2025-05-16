// Plik dla struktury Dictionary

use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom}; 
use std::path::{Path, PathBuf};
use std::sync::Arc; 

use morfologik_fsa::fsa_trait::Fsa;
use morfologik_fsa::fsa5::FSA5;
use morfologik_fsa::cfsa2::CFSA2;
// Używamy poprawnych stałych wersji
use morfologik_fsa::fsa_header::{VERSION_FSA5, VERSION_CFSA2, FsaHeader};
use morfologik_fsa::error::FsaError;


use crate::dictionary_metadata::{DictionaryMetadata, EncoderType, DictionaryAttribute};
use crate::encoders::{
    NoEncoder, TrimPrefixEncoder, TrimPrefixAndSuffixEncoder, TrimSuffixEncoder,
    TrimInfixAndSuffixEncoder,
};
use crate::sequence_encoder_trait::SequenceEncoder;
use crate::error::{Result as StemmingResult, StemmingError};

/// Reprezentuje słownik morfologiczny.
#[derive(Debug, Clone)] 
pub struct Dictionary {
    pub fsa: Arc<Box<dyn Fsa + Send + Sync>>, 
    pub metadata: Arc<DictionaryMetadata>, 
    pub encoder: Arc<Box<dyn SequenceEncoder + Send + Sync>>, 
}

impl Dictionary {
    pub fn from_file<P: AsRef<Path>>(dictionary_file_path: P) -> StemmingResult<Self> {
        let path_ref = dictionary_file_path.as_ref();

        let metadata = DictionaryMetadata::from_dictionary_path(path_ref)?;
        
        let dict_file = File::open(path_ref).map_err(|e| {
            StemmingError::Io(format!(
                "Failed to open dictionary file '{}': {}",
                path_ref.display(),
                e
            ))
        })?;
        let mut reader = BufReader::new(dict_file);

        Self::from_readers(&mut reader, metadata)
    }

    pub fn from_embedded(dict_fsa_bytes: &[u8], info_bytes: &[u8]) -> StemmingResult<Self> {
        let metadata = DictionaryMetadata::from_bytes(info_bytes)?;
        let mut reader = Cursor::new(dict_fsa_bytes); 
        Self::from_readers(&mut reader, metadata)
    }
    
    fn from_readers<R: Read + Seek>(reader: &mut R, metadata: DictionaryMetadata) -> StemmingResult<Self> {
        let arc_metadata = Arc::new(metadata);

        let original_pos = reader.stream_position().map_err(|e| StemmingError::Io(e.to_string()))?;
        
        let fsa_header_for_check = FsaHeader::read(reader)
            .map_err(StemmingError::Fsa)?;
        
        reader.seek(SeekFrom::Start(original_pos)).map_err(|e| StemmingError::Io(e.to_string()))?;

        let fsa_instance: Box<dyn Fsa + Send + Sync> = match fsa_header_for_check.version {
            VERSION_FSA5 => Box::new(FSA5::from_reader(reader).map_err(StemmingError::Fsa)?),
            // Teraz VERSION_CFSA2 to 0xC6
            VERSION_CFSA2 => {
                println!("[Dictionary::from_readers] Wykryto wersję CFSA2 (0xC6).");
                Box::new(CFSA2::from_reader(reader).map_err(StemmingError::Fsa)?)
            }
            // Usunięto gałąź dla VERSION_CFSA_LEGACY, ponieważ jest to teraz VERSION_CFSA2
            ver => return Err(StemmingError::Fsa(FsaError::UnsupportedVersion(ver))),
        };
        
        let arc_fsa = Arc::new(fsa_instance);

        let encoder_type_from_meta = arc_metadata.get_encoder()?;
        let separator = arc_metadata.get_separator()?;

        let sequence_encoder: Box<dyn SequenceEncoder + Send + Sync> = match encoder_type_from_meta {
            EncoderType::None => Box::new(NoEncoder::new()),
            EncoderType::Suffix => Box::new(TrimSuffixEncoder::new(separator as u8)),
            EncoderType::Prefix => Box::new(TrimPrefixEncoder::new(separator as u8)),
            EncoderType::Infix => Box::new(TrimInfixAndSuffixEncoder::new(separator as u8)),
        };
        let arc_encoder = Arc::new(sequence_encoder);

        Ok(Dictionary {
            fsa: arc_fsa,
            metadata: arc_metadata,
            encoder: arc_encoder,
        })
    }


    pub fn get_info_file_path<P: AsRef<Path>>(dictionary_file_path: P) -> PathBuf {
        dictionary_file_path.as_ref().with_extension("info")
    }

    #[allow(dead_code)]
    pub fn get_features_file_path<P: AsRef<Path>>(dictionary_file_path: P) -> PathBuf {
        dictionary_file_path.as_ref().with_extension("feat")
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use morfologik_fsa::fsa_header::FSA_MAGIC; 

    fn create_test_info_file_content(sep: char, enc_type: EncoderType, fsa_ver_str: &str) -> String {
        format!(
            "fsa.dict.separator = {}\n\
             fsa.dict.encoding = UTF-8\n\
             fsa.dict.encoder = {}\n\
             fsa.version = {}",
            sep, enc_type.as_str(), fsa_ver_str
        )
    }

    fn create_test_fsa5_dict_bytes() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.extend_from_slice(&[VERSION_FSA5]); 
        data.extend_from_slice(&[1]); 
        data.extend_from_slice(&[1]); 
        data.extend_from_slice(&[0u8, 0u8]); 
        data
    }
    
    // Ta funkcja teraz tworzy poprawny nagłówek dla CFSA2 (wersja 0xC6)
    fn create_test_cfsa2_dict_bytes() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.extend_from_slice(&[VERSION_CFSA2]); // Używa poprawionej VERSION_CFSA2 = 0xC6
        let fsa_flags_val = morfologik_fsa::fsa_header::FsaFlags::empty().bits();
        let gtl_info_val = 0u8; 
        let automaton_flags_short = (fsa_flags_val & 0x00FF) | ((gtl_info_val as u16) << 8);
        data.extend_from_slice(&automaton_flags_short.to_le_bytes());
        data
    }
    
    // Usunięto create_test_cfsa_legacy_dict_bytes, ponieważ jest to teraz create_test_cfsa2_dict_bytes


     fn create_test_info_file(content: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        temp_file.flush().unwrap();
        temp_file
    }

    fn create_test_fsa5_dict_file(path: &Path) {
        let mut file = File::create(path).unwrap();
        file.write_all(&create_test_fsa5_dict_bytes()).unwrap();
        file.flush().unwrap();
    }
    
    fn create_test_cfsa2_dict_file(path: &Path) {
         let mut file = File::create(path).unwrap();
        file.write_all(&create_test_cfsa2_dict_bytes()).unwrap();
        file.flush().unwrap();
    }

    #[test]
    fn test_dictionary_from_embedded_fsa5_no_encoder() -> StemmingResult<()> {
        let info_content_str = create_test_info_file_content('+', EncoderType::None, "FSA5");
        let dict_bytes_vec = create_test_fsa5_dict_bytes();
        let info_bytes_vec = info_content_str.into_bytes(); 

        let dict = Dictionary::from_embedded(&dict_bytes_vec, &info_bytes_vec)?;

        assert_eq!(dict.metadata.get_separator()?, '+');
        assert_eq!(dict.metadata.get_encoder()?, EncoderType::None);
        assert_eq!(dict.encoder.get_encoder_type(), EncoderType::None);
        assert_eq!(dict.fsa.get_root_node(), 0); 
        Ok(())
    }

    #[test]
    fn test_dictionary_from_embedded_cfsa2_suffix_encoder() -> StemmingResult<()> {
        // Ten test teraz używa create_test_cfsa2_dict_bytes, które generuje nagłówek z wersją 0xC6
        let info_content_str = create_test_info_file_content('|', EncoderType::Suffix, "CFSA2"); // .info może nadal mówić CFSA2
        let dict_bytes_vec = create_test_cfsa2_dict_bytes();
        let info_bytes_vec = info_content_str.into_bytes(); 
        
        let dict = Dictionary::from_embedded(&dict_bytes_vec, &info_bytes_vec)?;
        
        assert_eq!(dict.metadata.get_separator()?, '|');
        assert_eq!(dict.metadata.get_encoder()?, EncoderType::Suffix);
        assert_eq!(dict.encoder.get_encoder_type(), EncoderType::Suffix);
        assert_eq!(dict.fsa.get_root_node(), 0);
        Ok(())
    }

    // Usunięto test test_dictionary_from_embedded_cfsa_legacy, ponieważ jest on teraz pokryty przez
    // test_dictionary_from_embedded_cfsa2_suffix_encoder z poprawną wersją.


    #[test]
    fn test_dictionary_from_file_fsa5_no_encoder() -> StemmingResult<()> {
        let info_content = create_test_info_file_content('+', EncoderType::None, "FSA5");
        let info_file = create_test_info_file(&info_content);
        let dict_path = info_file.path().with_extension("dict");
        create_test_fsa5_dict_file(&dict_path);

        let dict = Dictionary::from_file(&dict_path)?;

        assert_eq!(dict.metadata.get_separator()?, '+');
        assert_eq!(dict.metadata.get_encoding(), "UTF-8"); 
        assert_eq!(dict.metadata.get_encoder()?, EncoderType::None);
        assert_eq!(dict.encoder.get_encoder_type(), EncoderType::None);
        assert_eq!(dict.fsa.get_root_node(), 0); 

        Ok(())
    }

    #[test]
    fn test_dictionary_from_file_cfsa2_suffix_encoder() -> StemmingResult<()> {
        let info_content = create_test_info_file_content('|', EncoderType::Suffix, "CFSA2");
        let info_file = create_test_info_file(&info_content);
        let dict_path = info_file.path().with_extension("dict");
        create_test_cfsa2_dict_file(&dict_path); // Tworzy plik z wersją 0xC6

        let dict = Dictionary::from_file(&dict_path)?;

        assert_eq!(dict.metadata.get_separator()?, '|');
        assert_eq!(dict.metadata.get_encoder()?, EncoderType::Suffix);
        assert_eq!(dict.encoder.get_encoder_type(), EncoderType::Suffix);
        assert_eq!(dict.fsa.get_root_node(), 0);

        Ok(())
    }

    #[test]
    fn test_dictionary_from_file_missing_info() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dict_path = temp_dir.path().join("test.dict");
        create_test_fsa5_dict_file(&dict_path);

        let result = Dictionary::from_file(&dict_path);
        assert!(matches!(result, Err(StemmingError::MetadataNotFound(_))));
    }

    #[test]
    fn test_dictionary_from_file_missing_dict() {
        let info_content = create_test_info_file_content(' ', EncoderType::None, "FSA5");
        let info_file = create_test_info_file(&info_content);
        let dict_path = info_file.path().with_extension("dict"); 

        let result = Dictionary::from_file(&dict_path);
        assert!(matches!(result, Err(StemmingError::Io(_))));
    }

    #[test]
    fn test_dictionary_from_file_unsupported_fsa_version_in_file() {
        let info_content = create_test_info_file_content(' ', EncoderType::None, "FSA_ANY"); 
        let info_file = create_test_info_file(&info_content);
        let dict_path_v3 = info_file.path().with_extension("dict");
        
        let mut file_v3 = File::create(&dict_path_v3).unwrap();
        file_v3.write_all(&FSA_MAGIC).unwrap();
        file_v3.write_all(&[3]).unwrap(); // Wersja 3, której nie obsługujemy
        file_v3.write_all(&[1,1,0,0]).unwrap(); 
        file_v3.flush().unwrap();

        let result = Dictionary::from_file(&dict_path_v3);
        assert!(matches!(result, Err(StemmingError::Fsa(FsaError::UnsupportedVersion(3)))));
    }
}
