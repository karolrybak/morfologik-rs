// Plik wygenerowany automatycznie przez skrypt.
// TODO: Dodać właściwą implementację.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor}; // Usunięto `self`
use std::path::Path;
use std::str::FromStr;

use crate::error::{Result, StemmingError};


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EncoderType {
    None,
    Prefix,
    Infix,
    Suffix,
}

impl FromStr for EncoderType {
    type Err = StemmingError; 

    fn from_str(s: &str) -> Result<Self> { 
        match s.trim().to_uppercase().as_str() {
            "NONE" => Ok(EncoderType::None),
            "PREFIX" => Ok(EncoderType::Prefix),
            "INFIX" => Ok(EncoderType::Infix),
            "SUFFIX" => Ok(EncoderType::Suffix),
            _ => Err(StemmingError::InvalidMetadataValue(format!(
                "Unknown EncoderType: {}",
                s
            ))),
        }
    }
}

impl EncoderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EncoderType::None => "NONE",
            EncoderType::Prefix => "PREFIX",
            EncoderType::Infix => "INFIX",
            EncoderType::Suffix => "SUFFIX",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DictionaryAttribute {
    Separator,
    Encoding,
    Encoder,
    FsaVersion,
    License,
    Author,
    CreationDate,
}

impl DictionaryAttribute {
    pub fn key_name(&self) -> &'static str {
        match self {
            DictionaryAttribute::Separator => "fsa.dict.separator",
            DictionaryAttribute::Encoding => "fsa.dict.encoding",
            DictionaryAttribute::Encoder => "fsa.dict.encoder",
            DictionaryAttribute::FsaVersion => "fsa.version",
            DictionaryAttribute::License => "license.key",
            DictionaryAttribute::Author => "author.key",
            DictionaryAttribute::CreationDate => "creation-date.key",
        }
    }

    #[allow(dead_code)] // Może być użyte w przyszłości
    pub fn from_key_name(key: &str) -> Option<Self> {
        match key {
            "fsa.dict.separator" => Some(DictionaryAttribute::Separator),
            "fsa.dict.encoding" => Some(DictionaryAttribute::Encoding),
            "fsa.dict.encoder" => Some(DictionaryAttribute::Encoder),
            "fsa.version" => Some(DictionaryAttribute::FsaVersion),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DictionaryMetadata {
    attributes: HashMap<String, String>,
}

impl DictionaryMetadata {
    pub const DEFAULT_SEPARATOR: char = '\t';
    pub const DEFAULT_ENCODING: &'static str = "UTF-8";
    pub const DEFAULT_ENCODER: EncoderType = EncoderType::Suffix;

    pub fn new() -> Self {
        DictionaryMetadata::default()
    }

    pub fn from_dictionary_path<P: AsRef<Path>>(dict_path: P) -> Result<Self> {
        let path_ref = dict_path.as_ref();
        let info_path = path_ref.with_extension("info");

        if !info_path.exists() {
            return Err(StemmingError::MetadataNotFound(
                info_path.to_string_lossy().into_owned(),
            ));
        }
        Self::from_info_file(&info_path)
    }

    pub fn from_info_file<P: AsRef<Path>>(info_file_path: P) -> Result<Self> {
        let file = File::open(info_file_path.as_ref()).map_err(|e| {
            StemmingError::Io(format!( 
                "Failed to open metadata file '{}': {}",
                info_file_path.as_ref().display(),
                e
            ))
        })?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    pub fn from_reader<R: BufRead>(reader: R) -> Result<Self> {
        let mut attributes = HashMap::new();
        for line_res in reader.lines() {
            let line = line_res.map_err(|e| StemmingError::Io(format!("Error reading metadata line: {}", e)))?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(eq_index) = line.find('=') {
                let key = line[..eq_index].trim().to_string();
                let value = line[eq_index + 1..].trim().to_string();
                attributes.insert(key, value);
            }
        }
        Ok(DictionaryMetadata { attributes })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let cursor = Cursor::new(bytes);
        let reader = BufReader::new(cursor);
        Self::from_reader(reader)
    }


    pub fn get_string(&self, attribute: DictionaryAttribute) -> Option<&String> {
        self.attributes.get(attribute.key_name())
    }
    
    fn get_string_or_default<'s>(&'s self, attribute_key: &str, default_value: &'s str) -> &'s str {
        self.attributes.get(attribute_key).map_or(default_value, |s| s.as_str())
    }

    pub fn get_separator(&self) -> Result<char> { 
        self.attributes
            .get(DictionaryAttribute::Separator.key_name())
            .map_or(Ok(Self::DEFAULT_SEPARATOR), |s| {
                if s.len() == 1 {
                    Ok(s.chars().next().unwrap())
                } else {
                    Err(StemmingError::InvalidMetadataValue(format!(
                        "Invalid separator value: '{}'. Expected a single character.",
                        s
                    )))
                }
            })
    }

    pub fn get_encoding(&self) -> &str {
        self.get_string_or_default(DictionaryAttribute::Encoding.key_name(), Self::DEFAULT_ENCODING)
    }

    pub fn get_encoder(&self) -> Result<EncoderType> { 
        self.attributes
            .get(DictionaryAttribute::Encoder.key_name())
            .map_or(Ok(Self::DEFAULT_ENCODER.clone()), |s| {
                EncoderType::from_str(s)
            })
    }
    
    pub fn set_attribute(&mut self, key: String, value: String) {
        self.attributes.insert(key, value);
    }

    pub fn set(&mut self, attribute: DictionaryAttribute, value: String) {
        self.attributes.insert(attribute.key_name().to_string(), value);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use assert_matches::assert_matches;

    #[test]
    fn test_dictionary_attribute_key_name() {
        assert_eq!(DictionaryAttribute::Separator.key_name(), "fsa.dict.separator");
        assert_eq!(DictionaryAttribute::Encoder.key_name(), "fsa.dict.encoder");
    }

    #[test]
    fn test_from_info_file_basic() -> std::result::Result<(), Box<dyn std::error::Error>> { 
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "fsa.dict.separator = ,")?;
        writeln!(temp_file, "fsa.dict.encoding = ISO-8859-1")?;
        writeln!(temp_file, "fsa.dict.encoder = PREFIX")?;
        writeln!(temp_file, "# This is a comment")?;
        writeln!(temp_file, "fsa.version = FSA5")?;
        temp_file.flush()?;

        let metadata = DictionaryMetadata::from_info_file(temp_file.path())?;

        assert_eq!(metadata.get_separator()?, ',');
        assert_eq!(metadata.get_encoding(), "ISO-8859-1");
        assert_eq!(metadata.get_encoder()?, EncoderType::Prefix);
        assert_eq!(metadata.get_string(DictionaryAttribute::FsaVersion), Some(&"FSA5".to_string()));
        Ok(())
    }

    #[test]
    fn test_from_bytes_basic() -> Result<()> {
        let info_content = "fsa.dict.separator = *\n\
                            fsa.dict.encoder = INFIX";
        let metadata = DictionaryMetadata::from_bytes(info_content.as_bytes())?;
        assert_eq!(metadata.get_separator()?, '*');
        assert_eq!(metadata.get_encoder()?, EncoderType::Infix);
        assert_eq!(metadata.get_encoding(), DictionaryMetadata::DEFAULT_ENCODING); 
        Ok(())
    }


    #[test]
    fn test_from_info_file_defaults() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "# Only comments")?;
        temp_file.flush()?;

        let metadata = DictionaryMetadata::from_info_file(temp_file.path())?;

        assert_eq!(metadata.get_separator()?, DictionaryMetadata::DEFAULT_SEPARATOR);
        assert_eq!(metadata.get_encoding(), DictionaryMetadata::DEFAULT_ENCODING);
        assert_eq!(metadata.get_encoder()?, DictionaryMetadata::DEFAULT_ENCODER);
        Ok(())
    }

    #[test]
    fn test_from_info_file_invalid_separator() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "fsa.dict.separator = too_long")?;
        temp_file.flush()?;

        let metadata_res = DictionaryMetadata::from_info_file(temp_file.path())?;
        let sep_res = metadata_res.get_separator();
        assert_matches!(sep_res, Err(StemmingError::InvalidMetadataValue(_)));
        Ok(())
    }

    #[test]
    fn test_from_info_file_unknown_encoder() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "fsa.dict.encoder = UNKNOWN_ENCODER")?;
        temp_file.flush()?;

        let metadata_res = DictionaryMetadata::from_info_file(temp_file.path())?;
        let enc_res = metadata_res.get_encoder();
        assert_matches!(enc_res, Err(StemmingError::InvalidMetadataValue(_)));
        Ok(())
    }
    
    #[test]
    fn test_from_dictionary_path_no_info_file() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp_dict_file = NamedTempFile::with_suffix(".dict")?;
        
        let metadata_res = DictionaryMetadata::from_dictionary_path(temp_dict_file.path());
        assert_matches!(metadata_res, Err(StemmingError::MetadataNotFound(_)));
        Ok(())
    }

    #[test]
    fn test_encoder_type_from_str() {
        assert_eq!(EncoderType::from_str("NONE").unwrap(), EncoderType::None);
        assert_eq!(EncoderType::from_str("prefix").unwrap(), EncoderType::Prefix);
        assert_eq!(EncoderType::from_str("  INFIX  ").unwrap(), EncoderType::Infix);
        assert_eq!(EncoderType::from_str("SuFfIx").unwrap(), EncoderType::Suffix);
        assert_matches!(EncoderType::from_str("INVALID"), Err(StemmingError::InvalidMetadataValue(_)));
    }

    #[test]
    fn test_set_attribute() {
        let mut metadata = DictionaryMetadata::new();
        metadata.set_attribute("custom.key".to_string(), "custom_value".to_string());
        assert_eq!(metadata.attributes.get("custom.key"), Some(&"custom_value".to_string()));

        metadata.set(DictionaryAttribute::Author, "Test Author".to_string());
        assert_eq!(metadata.get_string(DictionaryAttribute::Author), Some(&"Test Author".to_string()));
    }
}
