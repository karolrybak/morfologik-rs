// Plik wygenerowany automatycznie przez skrypt.
// TODO: Dodać właściwą implementację.

use thiserror::Error;

/// Typ Result używany w tym crate.
pub type Result<T> = std::result::Result<T, FsaError>;

/// Enum reprezentujący błędy, które mogą wystąpić podczas operacji na FSA.
#[derive(Debug, Error)]
pub enum FsaError {
    /// Błąd wejścia/wyjścia.
    #[error("Błąd I/O: {0}")]
    Io(#[from] std::io::Error),

    /// Nieprawidłowa magiczna sekwencja bajtów w nagłówku FSA.
    #[error("Nieprawidłowa magiczna sekwencja FSA.")]
    InvalidMagic,

    /// Nieobsługiwana wersja formatu FSA.
    #[error("Nieobsługiwana wersja FSA: {0}")]
    UnsupportedVersion(u8),

    /// Błąd podczas parsowania danych, np. nieoczekiwany format.
    #[error("Błąd parsowania: {0}")]
    ParsingError(String),

    /// Osiągnięto koniec danych wejściowych w nieoczekiwanym momencie.
    #[error("Nieoczekiwany koniec danych wejściowych.")]
    UnexpectedEof,

    /// Automat FSA jest uszkodzony lub w nieprawidłowym stanie.
    #[error("Automat FSA jest uszkodzony lub w nieprawidłowym stanie: {0}")]
    CorruptedFsa(String),

    /// Podany offset węzła jest nieprawidłowy (np. poza zakresem).
    #[error("Nieprawidłowy offset węzła: {0}")]
    InvalidNodeOffset(usize),

    /// Podany offset łuku jest nieprawidłowy (np. poza zakresem).
    #[error("Nieprawidłowy offset łuku: {0}")]
    InvalidArcOffset(usize),

    /// Nie znaleziono łuku dla podanej etykiety z danego węzła.
    #[error("Nie znaleziono łuku dla etykiety '{label}' z węzła o offsecie {node_offset}")]
    ArcNotFound { label: u8, node_offset: usize },

    /// Bieżący łuk jest ostatnim łukiem wychodzącym z węzła i nie ma następnego.
    #[error("Brak następnego łuku po łuku o offsecie {0} (jest ostatni).")]
    NoNextArc(usize),
}
