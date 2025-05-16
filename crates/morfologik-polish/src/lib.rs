// Główny plik dla crate morfologik-polish

pub mod polish_stemmer;
pub use polish_stemmer::PolishStemmer;

#[cfg(test)]
mod tests {
    use super::*; 
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir; 
    
    use morfologik_stemming::stemmer_trait::Stemmer;
    use morfologik_stemming::error::{Result as StemmingResult, StemmingError};


    fn create_test_info_file_in_dir(dir: &Path, filename: &str, content: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let mut temp_file = File::create(&file_path).expect("Failed to create temp info file");
        write!(temp_file, "{}", content).expect("Failed to write to temp info file");
        temp_file.flush().expect("Failed to flush temp info file");
        file_path
    }
    
    fn create_fsa_with_kot_subst(path: &Path) {
        use morfologik_fsa::fsa_header::{FSA_MAGIC, VERSION_FSA5, FsaFlags};

        let key = b"kot";
        let value_as_single_sequence = b"KOT+SUBST"; 

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
        
        let mut file = File::create(path).expect("Failed to create FSA file for test");
        file.write_all(&fsa_file_bytes).expect("Failed to write to FSA file");
        file.flush().expect("Failed to flush FSA file");
    }

    #[test]
    fn test_polish_stemmer_from_path_lookup() -> StemmingResult<()> {
        let temp_dir = tempdir().unwrap();
        let dict_filename = "test_pol_path.dict";
        let info_filename = "test_pol_path.info";

        let info_content = "fsa.dict.separator = +\n\
                            fsa.dict.encoder = NONE\n\
                            fsa.version = FSA5";
        
        create_test_info_file_in_dir(temp_dir.path(), info_filename, info_content);
        let dict_path = temp_dir.path().join(dict_filename);
        create_fsa_with_kot_subst(&dict_path);

        // Testujemy from_path
        let stemmer = PolishStemmer::from_path(&dict_path)?;

        let results_kot = stemmer.lookup(b"kot")?;
        assert_eq!(results_kot.len(), 1);
        let word_data_kot = &results_kot[0];
        assert_eq!(word_data_kot.word(), b"kot");
        assert_eq!(word_data_kot.stem().unwrap_or_default(), b"KOT");
        assert_eq!(word_data_kot.tag().unwrap_or_default(), b"SUBST");

        let results_pies = stemmer.lookup(b"pies")?;
        assert!(results_pies.is_empty());
        Ok(())
    }

    #[test]
    fn test_polish_stemmer_new_with_embedded() {
        // Ten test zakłada, że pliki słownika są poprawnie osadzone
        // i że ścieżki w `include_bytes!` w `polish_stemmer.rs` są prawidłowe.
        // Jeśli ten test zawiedzie, najpierw sprawdź te ścieżki.
        match PolishStemmer::new() {
            Ok(stemmer) => {
                // Spróbujmy odpytać o znane słowo, jeśli słownik nie jest pusty
                // To zależy od zawartości osadzonego słownika.
                // Na potrzeby tego testu, zakładamy, że słowo "dom" istnieje.
                // W rzeczywistości trzeba by znać zawartość osadzonego słownika.
                // Jeśli słownik jest duży, ten test może być wolny.
                // Można też po prostu sprawdzić, czy `get_dictionary_metadata` działa.
                let metadata = stemmer.get_dictionary_metadata();
                println!("Wczytano osadzony słownik, separator: {}", metadata.get_separator().unwrap_or('?'));
                
                // Przykład lookup (dostosuj do zawartości swojego `polish.dict`)
                // match stemmer.lookup(b"dom") {
                //     Ok(results) => {
                //         if results.is_empty() {
                //             println!("Ostrzeżenie: Słowo 'dom' nie znalezione w osadzonym słowniku.");
                //         } else {
                //             println!("Wyniki dla 'dom': {:?}", results);
                //         }
                //     }
                //     Err(e) => panic!("Błąd lookup dla 'dom' w osadzonym słowniku: {:?}", e),
                // }
            }
            Err(e) => {
                panic!("Nie udało się utworzyć PolishStemmer z osadzonego słownika: {:?}. \
                        Sprawdź ścieżki w include_bytes! w polish_stemmer.rs oraz czy pliki \
                        crates/morfologik-polish/resources/morfologik/stemming/polish/polish.dict \
                        i .info istnieją i są poprawne.", e);
            }
        }
    }

    #[test]
    fn test_polish_stemmer_from_path_fails_if_dict_missing() {
        let non_existent_path = PathBuf::from("hopefully_non_existent_dictionary_12345.dict");
        let result = PolishStemmer::from_path(&non_existent_path);
        assert!(matches!(result, Err(StemmingError::Io(_)) | Err(StemmingError::MetadataNotFound(_))));
    }
}
