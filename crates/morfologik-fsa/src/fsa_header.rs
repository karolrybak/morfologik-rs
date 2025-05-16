use bitflags::bitflags;
use std::io;
use std::fmt::Debug;

use crate::error::{FsaError, Result}; 

/// Magiczna sekwencja bajtów identyfikująca plik FSA.
pub const FSA_MAGIC: [u8; 4] = [b'\\', b'f', b's', b'a'];

/// Wersja dla formatu FSA5.
pub const VERSION_FSA5: u8 = 5;
/// Wersja dla formatu CFSA2 (zgodnie z kodem Morfologik Java CFSA2.java).
pub const VERSION_CFSA2: u8 = 0xC6; // 198 dziesiętnie

// Możemy usunąć lub zakomentować _VERSION_CFSA_DEPRECATED_JAVA, jeśli nie jest używana.
// pub const _VERSION_CFSA_DEPRECATED_JAVA: u8 = 3; 

bitflags! {
    /// Flagi opisujące atrybuty automatu FSA.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FsaFlags: u16 {
        const FLEXIBLE = 0b0000_0000_0000_0001;
        const NUMBERS = 0b0000_0000_0000_0010;
        // Poniższe flagi są przestarzałe w Javie
        const _NEXTBIT_DEPRECATED = 0b0000_0000_0000_0100;
        const _STOPBIT_DEPRECATED = 0b0000_0000_0000_1000;
        const _TAILBIT_DEPRECATED = 0b0000_0000_0001_0000;
    }
}

/// Nagłówek automatu FSA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsaHeader {
    pub version: u8,
}

impl FsaHeader {
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

        // Akceptujemy teraz poprawną wersję CFSA2
        match version {
            VERSION_FSA5 | VERSION_CFSA2 /* | _VERSION_CFSA_DEPRECATED_JAVA */ => Ok(FsaHeader { version }),
            _ => Err(FsaError::UnsupportedVersion(version)),
        }
    }

    pub fn get_version(&self) -> u8 {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use assert_matches::assert_matches;

    #[test]
    fn test_read_fsa_header_fsa5_valid() {
        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&FSA_MAGIC); 
        data.push(VERSION_FSA5); 
        data.extend_from_slice(&[0u8, 1, 2, 3, 4, 5]);
        let mut cursor = Cursor::new(data);
        let header = FsaHeader::read(&mut cursor).unwrap();
        assert_eq!(header.version, VERSION_FSA5);
    }

    #[test]
    fn test_read_fsa_header_cfsa2_valid() { // Ten test teraz sprawdza 0xC6
        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.push(VERSION_CFSA2); // Teraz to jest 0xC6
        let mut cursor = Cursor::new(data);
        let header = FsaHeader::read(&mut cursor).unwrap();
        assert_eq!(header.version, VERSION_CFSA2);
    }

    // Usunięto test dla VERSION_CFSA_LEGACY, ponieważ jest to teraz VERSION_CFSA2

    #[test]
    fn test_read_fsa_header_invalid_magic() {
        let data = vec![b'a', b'b', b'c', b'd', VERSION_FSA5];
        let mut cursor = Cursor::new(data);
        let result = FsaHeader::read(&mut cursor);
        assert_matches!(result.unwrap_err(), FsaError::InvalidMagic);
    }

    #[test]
    fn test_read_fsa_header_unsupported_version() {
        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.push(4); // Wersja 4 jest teraz nieobsługiwana, jeśli nie ma dla niej definicji
        let mut cursor = Cursor::new(data);
        let result = FsaHeader::read(&mut cursor);
        assert_matches!(result.unwrap_err(), FsaError::UnsupportedVersion(4));
    }

    #[test]
    fn test_read_fsa_header_incomplete_magic() {
        let data = vec![b'\\', b'f', b's']; 
        let mut cursor = Cursor::new(data);
        let result = FsaHeader::read(&mut cursor);
        assert_matches!(result.unwrap_err(), FsaError::UnexpectedEof);
    }

    #[test]
    fn test_read_fsa_header_incomplete_version() {
        let data = FSA_MAGIC.to_vec(); 
        let mut cursor = Cursor::new(data);
        let result = FsaHeader::read(&mut cursor);
        assert_matches!(result.unwrap_err(), FsaError::UnexpectedEof);
    }

    #[test]
    fn test_fsa_flags() {
        let flags = FsaFlags::FLEXIBLE | FsaFlags::NUMBERS;
        assert!(flags.contains(FsaFlags::FLEXIBLE));
        assert!(flags.contains(FsaFlags::NUMBERS));
    }
}
