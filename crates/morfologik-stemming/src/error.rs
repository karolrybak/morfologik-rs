// Plik dla błędów specyficznych dla crate morfologik-stemming

use thiserror::Error;

// Importujemy FsaError z crate morfologik_fsa.
// Ścieżka `crate::fsa::error::FsaError` zakłada, że `morfologik-fsa`
// jest dostępne jako `morfologik_fsa` (nazwa w Cargo.toml) i że
// jego moduł błędów jest `morfologik_fsa::error::FsaError`.
// Jeśli struktura projektu lub nazwy są inne, trzeba to dostosować.
// W naszym przypadku, jeśli `morfologik-fsa` jest w tym samym workspace,
// a jego `lib.rs` eksportuje `pub mod error;`, to ścieżka może być
// `morfologik_fsa::error::FsaError`.
// Dla uproszczenia, jeśli `morfologik-fsa` jest zależnością, użyjemy nazwy crate.
// Zakładając, że w Cargo.toml dla morfologik-stemming mamy:
// morfologik-fsa = { path = "../morfologik-fsa" }
// to możemy użyć:
use morfologik_fsa::error::FsaError;


/// Typ Result używany w tym crate.
pub type Result<T> = std::result::Result<T, StemmingError>;

/// Enum reprezentujący błędy, które mogą wystąpić podczas operacji stemmingu i obsługi słowników.
#[derive(Debug, Error)]
pub enum StemmingError {
    /// Błąd wejścia/wyjścia, często związany z odczytem plików.
    #[error("Błąd I/O: {0}")]
    Io(String), // Przechowuje sformatowany komunikat błędu I/O dla większej elastyczności

    /// Nie znaleziono pliku metadanych (`.info`).
    #[error("Nie znaleziono pliku metadanych: {0}")]
    MetadataNotFound(String),

    /// Nieprawidłowa wartość w pliku metadanych.
    #[error("Nieprawidłowa wartość w metadanych: {0}")]
    InvalidMetadataValue(String),

    /// Błąd parsowania liczby (np. wersji) w metadanych.
    #[error("Błąd parsowania liczby w metadanych dla klucza '{key}': {value}, błąd: {source}")]
    MetadataParseIntError {
        key: String,
        value: String,
        #[source]
        source: std::num::ParseIntError,
    },

    /// Nieznany lub nieobsługiwany atrybut w metadanych.
    #[error("Nieznany atrybut metadanych: {0}")]
    UnknownMetadataAttribute(String),

    /// Błąd pochodzący z operacji na automacie FSA.
    #[error("Błąd FSA: {0}")]
    Fsa(#[from] FsaError), // Używamy FsaError z crate morfologik_fsa

    /// Nie można zmapować sekwencji wejściowej (np. słowa) na bajty słownika
    /// przy użyciu podanego kodowania znaków.
    #[error("Nie można zmapować sekwencji wejściowej na bajty słownika (charset: {charset}): '{unmappable_sequence}'")]
    UnmappableInput {
        charset: String,
        unmappable_sequence: String,
    },

    /// Ogólny błąd konfiguracji słownika.
    #[error("Błąd konfiguracji słownika: {0}")]
    DictionaryConfigurationError(String),

    /// Nie znaleziono słowa w słowniku.
    #[error("Słowo nie zostało znalezione w słowniku: '{word}'")]
    WordNotFound { word: String },

    /// Błąd podczas dekodowania sekwencji (np. formy podstawowej, tagu).
    #[error("Błąd dekodowania sekwencji: {0}")]
    SequenceDecodingError(String),

    /// Nieoczekiwany format danych w pliku słownika lub automacie.
    #[error("Nieoczekiwany format danych: {0}")]
    UnexpectedDataFormat(String),
}

// Implementacja konwersji z std::io::Error dla wygody,
// aby móc używać operatora `?` na wynikach operacji I/O.
// Zamiast tego, w kodzie można używać .map_err(|e| StemmingError::Io(e.to_string()))
impl From<std::io::Error> for StemmingError {
    fn from(err: std::io::Error) -> Self {
        StemmingError::Io(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_stemming_error_display() {
        let io_err_msg = "Plik nie istnieje".to_string();
        let io_std_err = io::Error::new(io::ErrorKind::NotFound, io_err_msg.clone());
        let stemming_io_err: StemmingError = io_std_err.into();
        assert_eq!(format!("{}", stemming_io_err), format!("Błąd I/O: {}", io_err_msg));

        let meta_not_found_err = StemmingError::MetadataNotFound("test.info".to_string());
        assert_eq!(format!("{}", meta_not_found_err), "Nie znaleziono pliku metadanych: test.info");

        let invalid_meta_val_err = StemmingError::InvalidMetadataValue("Zła wartość enkodera".to_string());
        assert_eq!(format!("{}", invalid_meta_val_err), "Nieprawidłowa wartość w metadanych: Zła wartość enkodera");

        // Test dla FsaError (zakładając, że FsaError::InvalidMagic istnieje i ma taki format)
        // Aby to przetestować, potrzebowalibyśmy rzeczywistego FsaError.
        // Możemy stworzyć prosty mock lub użyć `format!` do symulacji.
        // Załóżmy, że FsaError::InvalidMagic.to_string() daje "Nieprawidłowa magiczna sekwencja FSA."
        // let fsa_err_source = morfologik_fsa::error::FsaError::InvalidMagic;
        // let stemming_fsa_err = StemmingError::Fsa(fsa_err_source);
        // assert_eq!(format!("{}", stemming_fsa_err), "Błąd FSA: Nieprawidłowa magiczna sekwencja FSA.");
    }

    #[test]
    fn test_metadata_parse_int_error_display() {
        let parse_err = "12a".parse::<i32>().unwrap_err();
        let err = StemmingError::MetadataParseIntError {
            key: "fsa.version.value".to_string(),
            value: "12a".to_string(),
            source: parse_err,
        };
        assert!(format!("{}", err).contains("Błąd parsowania liczby w metadanych dla klucza 'fsa.version.value': 12a"));
        assert!(format!("{}", err).contains("invalid digit found in string"));
    }
}

