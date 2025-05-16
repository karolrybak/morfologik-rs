// Plik wygenerowany automatycznie przez skrypt.
// TODO: Dodać właściwą implementację.

use bitflags::bitflags;
use std::io;

// Import modułu error, który zdefiniujemy później
use crate::error::{FsaError, Result}; // Odkomentowane i dodane Result

/// Magiczna sekwencja bajtów identyfikująca plik FSA.
/// Odpowiada `FSA_MAGIC` z `FSA.java` (który jest tam '\\fsa')
pub const FSA_MAGIC: [u8; 4] = [b'\\', b'f', b's', b'a'];

/// Wersja dla formatu FSA5.
pub const VERSION_FSA5: u8 = 5;
/// Wersja dla formatu CFSA2.
pub const VERSION_CFSA2: u8 = 4;
/// Wersja dla przestarzałego formatu CFSA.
pub const _VERSION_CFSA_DEPRECATED: u8 = 3; // Zaznaczamy jako prywatny i deprecated

bitflags! {
    /// Flagi opisujące atrybuty automatu FSA.
    /// Odpowiada `morfologik.fsa.FSAFlags`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FsaFlags: u16 {
        /// Flaga wskazująca, że graf jest elastyczny (niektóre łuki mogą być pominięte).
        /// Wartość: 0x0001
        const FLEXIBLE = 0b0000_0000_0000_0001;

        /// Flaga wskazująca, że automat zawiera numery łuków (sekwencje są numerowane).
        /// Wartość: 0x0002
        const NUMBERS = 0b0000_0000_0000_0010;

        // Pola poniżej są oznaczone jako @deprecated w kodzie Java.
        // Zachowujemy je dla kompletności, ale można rozważyć ich usunięcie
        // lub oznaczenie jako deprecated również w Rust.

        /// @deprecated Nie używane.
        /// Wartość: 0x0004
        const _NEXTBIT_DEPRECATED = 0b0000_0000_0000_0100;
        /// @deprecated Nie używane.
        /// Wartość: 0x0008
        const _STOPBIT_DEPRECATED = 0b0000_0000_0000_1000;
        /// @deprecated Nie używane.
        /// Wartość: 0x0010
        const _TAILBIT_DEPRECATED = 0b0000_0000_0001_0000;
    }
}

/// Nagłówek automatu FSA.
/// Zawiera podstawowe informacje o automacie, takie jak wersja formatu.
/// Odpowiada `morfologik.fsa.FSAHeader`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsaHeader {
    /// Wersja formatu FSA.
    pub version: u8,
    // W oryginalnym FSAHeader.java nie ma bezpośrednio pola na flagi.
    // Flagi są częścią specyficznych formatów jak FSA5.
    // Możemy dodać pole `flags: Option<FsaFlags>` jeśli okaże się to przydatne
    // na tym poziomie, lub obsługiwać je w strukturach dla konkretnych wersji FSA.
}

impl FsaHeader {
    /// Odczytuje nagłówek FSA z podanego strumienia wejściowego.
    /// Sprawdza magiczną sekwencję i odczytuje wersję.
    ///
    /// # Błędy
    ///
    /// Zwraca `FsaError` jeśli wystąpi problem z odczytem,
    /// magiczna sekwencja jest nieprawidłowa lub wersja jest nieobsługiwana.
    pub fn read<R: io::Read>(reader: &mut R) -> Result<Self> {
        let mut magic_buffer = [0u8; 4];
        reader.read_exact(&mut magic_buffer).map_err(|e| {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                FsaError::UnexpectedEof
            } else {
                FsaError::Io(e)
            }
        })?;

        if magic_buffer != FSA_MAGIC {
            return Err(FsaError::InvalidMagic);
        }

        let mut version_buffer = [0u8; 1];
        reader.read_exact(&mut version_buffer).map_err(|e| {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                FsaError::UnexpectedEof
            } else {
                FsaError::Io(e)
            }
        })?;
        let version = version_buffer[0];

        match version {
            VERSION_FSA5 | VERSION_CFSA2 | _VERSION_CFSA_DEPRECATED => Ok(FsaHeader { version }),
            _ => Err(FsaError::UnsupportedVersion(version)),
        }
    }

    /// Zwraca wersję formatu FSA.
    pub fn get_version(&self) -> u8 {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    // Użyj assert_matches do sprawdzania wariantów enumów błędów
    use assert_matches::assert_matches;


    #[test]
    fn test_read_fsa_header_fsa5_valid() {
        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&FSA_MAGIC); // magic
        data.push(VERSION_FSA5); // version
        // Dodajmy trochę "ciała" automatu, żeby symulować pełniejszy plik
        data.extend_from_slice(&[0u8, 1, 2, 3, 4, 5]);


        let mut cursor = Cursor::new(data);
        let header_result = FsaHeader::read(&mut cursor);
        assert!(header_result.is_ok(), "Oczekiwano poprawnego odczytu nagłówka, otrzymano: {:?}", header_result.err());
        let header = header_result.unwrap();


        assert_eq!(header.version, VERSION_FSA5);
        assert_eq!(header.get_version(), VERSION_FSA5);

        // Sprawdźmy, czy kursor przesunął się o odpowiednią liczbę bajtów (4 magic + 1 version)
        assert_eq!(cursor.position(), 5);
    }

    #[test]
    fn test_read_fsa_header_cfsa2_valid() {
        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.push(VERSION_CFSA2);
        let mut cursor = Cursor::new(data);
        let header = FsaHeader::read(&mut cursor).unwrap();
        assert_eq!(header.version, VERSION_CFSA2);
    }

    #[test]
    fn test_read_fsa_header_invalid_magic() {
        let data = vec![b'a', b'b', b'c', b'd', VERSION_FSA5];
        let mut cursor = Cursor::new(data);
        let result = FsaHeader::read(&mut cursor);
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), FsaError::InvalidMagic);
    }

    #[test]
    fn test_read_fsa_header_unsupported_version() {
        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.push(10); // Jakaś nieobsługiwana wersja
        let mut cursor = Cursor::new(data);
        let result = FsaHeader::read(&mut cursor);
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), FsaError::UnsupportedVersion(10));
    }

    #[test]
    fn test_read_fsa_header_incomplete_magic() {
        let data = vec![b'\\', b'f', b's']; // Za krótkie
        let mut cursor = Cursor::new(data);
        let result = FsaHeader::read(&mut cursor);
        assert!(result.is_err());
        // Sprawdzamy konkretny błąd FsaError::UnexpectedEof
        assert_matches!(result.unwrap_err(), FsaError::UnexpectedEof);
    }

    #[test]
    fn test_read_fsa_header_incomplete_version() {
        let data = FSA_MAGIC.to_vec(); // Tylko magic, brak wersji
        let mut cursor = Cursor::new(data);
        let result = FsaHeader::read(&mut cursor);
        assert!(result.is_err());
        // Sprawdzamy konkretny błąd FsaError::UnexpectedEof
        assert_matches!(result.unwrap_err(), FsaError::UnexpectedEof);
    }

    #[test]
    fn test_fsa_flags() {
        let flags = FsaFlags::FLEXIBLE | FsaFlags::NUMBERS;
        assert!(flags.contains(FsaFlags::FLEXIBLE));
        assert!(flags.contains(FsaFlags::NUMBERS));
        assert!(!flags.contains(FsaFlags::_NEXTBIT_DEPRECATED));

        let flex_val: u16 = 0x0001;
        assert_eq!(FsaFlags::FLEXIBLE.bits(), flex_val);

        let num_val: u16 = 0x0002;
        assert_eq!(FsaFlags::NUMBERS.bits(), num_val);
    }
}
