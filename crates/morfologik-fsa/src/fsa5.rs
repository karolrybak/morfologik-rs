// Plik wygenerowany automatycznie przez skrypt.
// TODO: Dodać właściwą implementację.

use std::fs::File;
use std::io::{self, BufReader, Read, Seek};
use std::path::Path;


use crate::error::{FsaError, Result};
use crate::fsa_header::{FsaFlags, FsaHeader, VERSION_FSA5};
use crate::fsa_trait::{ArcOffset, Fsa, Node};

/// Implementacja automatu w wersji FSA5.
///
/// Format FSA5 jest następujący:
///
/// ```text
/// ---- FSA_MAGIC ----
/// byte             magic[0] = '\\';
/// byte             magic[1] = 'f';
/// byte             magic[2] = 's';
/// byte             magic[3] = 'a';
///
/// ---- HEADER ----
/// byte             FSA_VERSION = 5;
/// byte             gtl;        // node data address size (target, label)
/// byte             n_size;     // node data address size (next)
///
/// ---- FLAGS ----
/// unsigned short   flags;      // automaton flags (see FSAFlags)
///
/// ---- DATA ----
/// byte[]           arcs;       // arcs data
/// ```
///
/// Szczegóły dotyczące struktury łuków (arcs):
/// Każdy łuk (arc) w `arcs` ma następującą strukturę:
/// - `label` (1 bajt): etykieta łuku.
/// - `flags` (1 bajt): flagi łuku:
///     - `BIT_ARC_FINAL` (0x01): czy łuk jest końcowy.
///     - `BIT_ARC_LAST` (0x02): czy łuk jest ostatnim z danego węzła.
///     - `BIT_ARC_NEXT` (0x04): czy łuk ma zakodowany adres następnego łuku (jeśli nie, to jest on bezpośrednio po nim).
///     - `BIT_ARC_TARGET_NEXT` (0x08): jeśli ustawione, adres docelowy jest adresem następnego *węzła*. W tej implementacji (zgodnie z FSA5.java) adresy docelowe są zawsze absolutne.
/// - `target_address` (zmienna długość, `gtl` bajtów): adres węzła docelowego.
/// - `next_arc_address` (zmienna długość, `n_size` bajtów, opcjonalnie): adres następnego łuku, jeśli flaga `BIT_ARC_NEXT` jest ustawiona.
///
/// Adresy mogą być względne lub bezwzględne w zależności od implementacji (w FSA5 są to offsety).
#[derive(Debug)]
pub struct FSA5 {
    /// Wersja automatu (powinna być VERSION_FSA5).
    pub version: u8,
    /// Flagi automatu.
    pub flags: FsaFlags,
    /// Rozmiar adresu dla (target, label) w bajtach.
    /// W FSA5.java to `gtl = gotoLength`.
    pub goto_length: u8,
    /// Rozmiar adresu dla następnego łuku w bajtach.
    /// W FSA5.java to `n_size = nodeDataLength`.
    pub node_data_length: u8,
    /// Surowe dane łuków automatu.
    pub arcs_data: Vec<u8>,
    /// Adres (offset) korzenia automatu w `arcs_data`.
    /// W FSA5.java to `rootNodeOffset`.
    pub root_node_offset: usize,
}

// Flagi bitowe dla pojedynczego łuku w FSA5
pub const BIT_ARC_FINAL: u8 = 0x01;
pub const BIT_ARC_LAST: u8 = 0x02;
pub const BIT_ARC_NEXT: u8 = 0x04;
pub const BIT_ARC_TARGET_NEXT: u8 = 0x08;
#[allow(dead_code)]
pub const BIT_TARGET_RELATIVE: u8 = 0x10;

/// Mapuje błąd `std::io::Error` na `FsaError`, specyficznie obsługując `UnexpectedEof`.
fn map_io_error(e: std::io::Error) -> FsaError {
    if e.kind() == io::ErrorKind::UnexpectedEof {
        FsaError::UnexpectedEof
    } else {
        FsaError::Io(e)
    }
}

impl FSA5 {
    /// Wczytuje automat FSA5 z podanej ścieżki do pliku.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).map_err(FsaError::Io)?;
        let mut reader = BufReader::new(file);
        Self::from_reader(&mut reader)
    }

    /// Wczytuje automat FSA5 z podanego czytnika (readera).
    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let header = FsaHeader::read(reader)?;
        if header.version != VERSION_FSA5 {
            return Err(FsaError::UnsupportedVersion(header.version));
        }

        let mut gtl_buffer = [0u8; 1];
        reader.read_exact(&mut gtl_buffer).map_err(map_io_error)?;
        let goto_length = gtl_buffer[0];

        let mut n_size_buffer = [0u8; 1];
        reader.read_exact(&mut n_size_buffer).map_err(map_io_error)?;
        let node_data_length = n_size_buffer[0];

        let mut flags_buffer = [0u8; 2];
        reader.read_exact(&mut flags_buffer).map_err(map_io_error)?;
        let flags_val = u16::from_le_bytes(flags_buffer);
        let flags = FsaFlags::from_bits_truncate(flags_val);

        let root_node_offset = if flags.contains(FsaFlags::NUMBERS) {
            if goto_length == 0 {
                return Err(FsaError::CorruptedFsa("goto_length is 0 with NUMBERS flag, cannot read root_node_offset".to_string()));
            }
            let mut root_offset_buffer = vec![0u8; goto_length as usize];
            reader.read_exact(&mut root_offset_buffer).map_err(map_io_error)?;
            read_address(&root_offset_buffer, goto_length)?
        } else {
            0
        };

        let mut arcs_data = Vec::new();
        reader.read_to_end(&mut arcs_data).map_err(map_io_error)?;

        Ok(FSA5 {
            version: header.version,
            flags,
            goto_length,
            node_data_length,
            arcs_data,
            root_node_offset,
        })
    }

    /// Pomocnicza funkcja do odczytu adresu o zmiennej długości z `arcs_data`.
    #[inline(always)]
    fn read_address_at(&self, offset: usize, length: u8) -> Result<usize> {
        if length == 0 {
            return Ok(0);
        }
        let end_offset = offset.checked_add(length as usize).ok_or_else(|| FsaError::CorruptedFsa(format!("Address offset overflow: {} + {}", offset, length)))?;

        if end_offset > self.arcs_data.len() {
            return Err(FsaError::CorruptedFsa(format!(
                "Attempt to read address beyond arcs_data bounds: offset {}, length {}, data_len {}",
                offset, length, self.arcs_data.len()
            )));
        }
        read_address(&self.arcs_data[offset..end_offset], length)
    }
}

/// Pomocnicza funkcja do odczytu adresu (offsetu) o zmiennej długości z bufora.
/// Adresy są zapisywane w formacie little-endian.
fn read_address(buffer: &[u8], length: u8) -> Result<usize> {
    if length == 0 {
        return Ok(0);
    }
    if buffer.len() < length as usize {
        return Err(FsaError::CorruptedFsa(format!(
            "Buffer too short ({} bytes) to read address of length {}",
            buffer.len(),
            length
        )));
    }

    let mut address: usize = 0;
    for i in 0..(length as usize) {
        address |= (buffer[i] as usize) << (i * 8);
    }
    Ok(address)
}


impl Fsa for FSA5 {
    fn get_flags(&self) -> FsaFlags {
        self.flags
    }

    fn get_root_node(&self) -> Node {
        self.root_node_offset
    }

    fn get_first_arc(&self, node_offset: Node) -> Result<ArcOffset> {
        if node_offset >= self.arcs_data.len() && self.arcs_data.is_empty() && node_offset == 0 {
            return Err(FsaError::InvalidNodeOffset(node_offset));
        }
        if node_offset >= self.arcs_data.len() {
            return Err(FsaError::InvalidNodeOffset(node_offset));
        }
        Ok(node_offset)
    }

    fn get_next_arc(&self, current_arc_offset: ArcOffset) -> Result<ArcOffset> {
        let arc_flags_byte_offset = current_arc_offset.checked_add(1).ok_or(FsaError::InvalidArcOffset(current_arc_offset))?;
        if arc_flags_byte_offset >= self.arcs_data.len() {
            return Err(FsaError::InvalidArcOffset(current_arc_offset));
        }
        let arc_flags = self.arcs_data[arc_flags_byte_offset];

        if (arc_flags & BIT_ARC_LAST) != 0 {
            return Err(FsaError::NoNextArc(current_arc_offset));
        }

        if (arc_flags & BIT_ARC_NEXT) != 0 {
            let next_arc_address_offset_start = arc_flags_byte_offset.checked_add(1).ok_or(FsaError::InvalidArcOffset(current_arc_offset))?
                                                .checked_add(self.goto_length as usize).ok_or(FsaError::InvalidArcOffset(current_arc_offset))?;
            self.read_address_at(next_arc_address_offset_start, self.node_data_length)
        } else {
            let next_arc_offset = arc_flags_byte_offset.checked_add(1).ok_or(FsaError::InvalidArcOffset(current_arc_offset))?
                                  .checked_add(self.goto_length as usize).ok_or(FsaError::InvalidArcOffset(current_arc_offset))?;
            if next_arc_offset > self.arcs_data.len() {
                 return Err(FsaError::CorruptedFsa(format!("Calculated next arc offset {} is out of bounds {}", next_arc_offset, self.arcs_data.len())));
            }
            Ok(next_arc_offset)
        }
    }

    fn get_arc(&self, node_offset: Node, label: u8) -> Result<ArcOffset> {
        if node_offset >= self.arcs_data.len() && !(self.arcs_data.is_empty() && node_offset == 0) {
            return Err(FsaError::InvalidNodeOffset(node_offset));
        }
        if self.arcs_data.is_empty() && node_offset == 0 {
             return Err(FsaError::ArcNotFound { label, node_offset });
        }


        let mut current_arc_result = self.get_first_arc(node_offset);

        while let Ok(arc_offset) = current_arc_result {
            if arc_offset >= self.arcs_data.len() {
                return Err(FsaError::CorruptedFsa(format!("ArcOffset offset {} out of bounds while searching for label {}", arc_offset, label)));
            }
            let arc_label = self.arcs_data[arc_offset];

            if arc_label == label {
                return Ok(arc_offset);
            }
            if arc_label > label {
                break;
            }

            let arc_flags_byte_offset = arc_offset.checked_add(1).ok_or(FsaError::InvalidArcOffset(arc_offset))?;
            if arc_flags_byte_offset >= self.arcs_data.len() {
                 return Err(FsaError::CorruptedFsa(format!("ArcOffset flags offset {} out of bounds for arc {}", arc_flags_byte_offset, arc_offset)));
            }
            let arc_flags = self.arcs_data[arc_flags_byte_offset];
            if (arc_flags & BIT_ARC_LAST) != 0 {
                break;
            }
            current_arc_result = self.get_next_arc(arc_offset);
        }
        Err(FsaError::ArcNotFound { label, node_offset })
    }

    fn get_end_node(&self, arc_offset: ArcOffset) -> Result<Node> {
        let target_address_start_offset = arc_offset.checked_add(1).ok_or(FsaError::InvalidArcOffset(arc_offset))?
                                          .checked_add(1).ok_or(FsaError::InvalidArcOffset(arc_offset))?;

        let target_node_raw = self.read_address_at(target_address_start_offset, self.goto_length)?;

        Ok(target_node_raw)
    }

    fn get_arc_label(&self, arc_offset: ArcOffset) -> Result<u8> {
        if arc_offset < self.arcs_data.len() {
            Ok(self.arcs_data[arc_offset])
        } else {
            Err(FsaError::InvalidArcOffset(arc_offset))
        }
    }

    fn is_arc_final(&self, arc_offset: ArcOffset) -> Result<bool> {
        let arc_flags_offset = arc_offset.checked_add(1).ok_or(FsaError::InvalidArcOffset(arc_offset))?;
        if arc_flags_offset < self.arcs_data.len() {
            Ok((self.arcs_data[arc_flags_offset] & BIT_ARC_FINAL) != 0)
        } else {
            Err(FsaError::InvalidArcOffset(arc_offset))
        }
    }

    fn is_arc_terminal(&self, arc_offset: ArcOffset) -> Result<bool> {
        let arc_flags_offset = arc_offset.checked_add(1).ok_or(FsaError::InvalidArcOffset(arc_offset))?;
        if arc_flags_offset < self.arcs_data.len() {
            Ok((self.arcs_data[arc_flags_offset] & BIT_ARC_LAST) != 0)
        } else {
            Err(FsaError::InvalidArcOffset(arc_offset))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*; // Daje dostęp do BIT_ARC_FINAL etc. z tego modułu
    use std::io::Cursor;
    use crate::fsa_header::FSA_MAGIC;
    use assert_matches::assert_matches;

    fn create_test_fsa5_ab_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.push(VERSION_FSA5);
        data.push(1); 
        data.push(1); 
        data.extend_from_slice(&0u16.to_le_bytes()); 

        data.push(b'a');
        data.push(BIT_ARC_FINAL); // Używamy stałej z tego modułu (fsa5.rs)
        data.push(0x00);

        data.push(b'b');
        data.push(BIT_ARC_FINAL | BIT_ARC_LAST); // Używamy stałych z tego modułu
        data.push(0x00);
        data
    }

    #[test]
    fn test_fsa5_from_reader_simple_ab() {
        let fsa_file_bytes = create_test_fsa5_ab_data();
        let mut cursor = Cursor::new(fsa_file_bytes.clone());
        let fsa = FSA5::from_reader(&mut cursor).expect("Failed to read FSA5");

        assert_eq!(fsa.version, VERSION_FSA5);
        assert_eq!(fsa.flags, FsaFlags::empty());
        assert_eq!(fsa.goto_length, 1);
        assert_eq!(fsa.node_data_length, 1);
        assert_eq!(fsa.root_node_offset, 0);

        let header_len = 4 + 1 + 1 + 1 + 2;
        let expected_arcs_data = &fsa_file_bytes[header_len..];
        assert_eq!(fsa.arcs_data, expected_arcs_data);
    }

    #[test]
    fn test_fsa5_get_root_node() {
        let fsa_file_bytes = create_test_fsa5_ab_data();
        let mut cursor = Cursor::new(fsa_file_bytes);
        let fsa = FSA5::from_reader(&mut cursor).unwrap();
        assert_eq!(fsa.get_root_node(), 0);
    }

    #[test]
    fn test_fsa5_get_first_arc() {
        let fsa_file_bytes = create_test_fsa5_ab_data();
        let mut cursor = Cursor::new(fsa_file_bytes);
        let fsa = FSA5::from_reader(&mut cursor).unwrap();

        let root_node = fsa.get_root_node();
        let first_arc = fsa.get_first_arc(root_node).unwrap();
        assert_eq!(first_arc, 0);
        assert_eq!(fsa.get_arc_label(first_arc).unwrap(), b'a');
    }

    #[test]
    fn test_fsa5_get_next_arc() {
        let fsa_file_bytes = create_test_fsa5_ab_data();
        let mut cursor = Cursor::new(fsa_file_bytes);
        let fsa = FSA5::from_reader(&mut cursor).unwrap();

        let root_node = fsa.get_root_node();
        let arc_a = fsa.get_first_arc(root_node).unwrap();
        
        let arc_b = fsa.get_next_arc(arc_a).unwrap();
        assert_eq!(arc_b, 3);
        assert_eq!(fsa.get_arc_label(arc_b).unwrap(), b'b');

        assert!(fsa.is_arc_terminal(arc_b).unwrap());
        let next_after_b_result = fsa.get_next_arc(arc_b);
        assert_matches!(next_after_b_result, Err(FsaError::NoNextArc(3)));
    }

     #[test]
    fn test_fsa5_get_arc() {
        let fsa_file_bytes = create_test_fsa5_ab_data();
        let mut cursor = Cursor::new(fsa_file_bytes);
        let fsa = FSA5::from_reader(&mut cursor).unwrap();
        let root = fsa.get_root_node();

        let arc_a = fsa.get_arc(root, b'a').unwrap();
        assert_eq!(fsa.get_arc_label(arc_a).unwrap(), b'a');
        assert_eq!(arc_a, 0);

        let arc_b = fsa.get_arc(root, b'b').unwrap();
        assert_eq!(fsa.get_arc_label(arc_b).unwrap(), b'b');
        assert_eq!(arc_b, 3);

        let result_c = fsa.get_arc(root, b'c');
        assert_matches!(result_c, Err(FsaError::ArcNotFound { label: b'c', node_offset: 0 }));
    }

    #[test]
    fn test_fsa5_arc_properties() {
        let fsa_file_bytes = create_test_fsa5_ab_data();
        let mut cursor = Cursor::new(fsa_file_bytes);
        let fsa = FSA5::from_reader(&mut cursor).unwrap();
        let root = fsa.get_root_node();

        let arc_a = fsa.get_arc(root, b'a').unwrap();
        assert!(fsa.is_arc_final(arc_a).unwrap());
        assert!(!fsa.is_arc_terminal(arc_a).unwrap());
        assert_eq!(fsa.get_end_node(arc_a).unwrap(), 0);

        let arc_b = fsa.get_arc(root, b'b').unwrap();
        assert!(fsa.is_arc_final(arc_b).unwrap());
        assert!(fsa.is_arc_terminal(arc_b).unwrap());
        assert_eq!(fsa.get_end_node(arc_b).unwrap(), 0);
    }

    fn create_test_fsa5_with_numbers_data(root_offset_val: usize, goto_len: u8) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.push(VERSION_FSA5);
        data.push(goto_len);
        data.push(1);
        data.extend_from_slice(&FsaFlags::NUMBERS.bits().to_le_bytes());

        let mut root_offset_bytes = vec![0u8; goto_len as usize];
        if goto_len > 0 {
            for i in 0..(goto_len as usize) {
                root_offset_bytes[i] = ((root_offset_val >> (i * 8)) & 0xFF) as u8;
            }
        }
        data.extend_from_slice(&root_offset_bytes);
        
        if root_offset_val > 0 && root_offset_val < 255 && goto_len > 0 {
            for _ in 0..=root_offset_val {
                 data.push(0);
            }
        }
        data.push(b'x'); data.push(BIT_ARC_FINAL | BIT_ARC_LAST); data.push(0);


        data
    }

    #[test]
    fn test_fsa5_from_reader_with_numbers_flag() {
        let root_offset_val = 0x0A;
        let goto_len = 1;
        let fsa_file_bytes = create_test_fsa5_with_numbers_data(root_offset_val, goto_len);
        let mut cursor = Cursor::new(fsa_file_bytes.clone());
        let fsa = FSA5::from_reader(&mut cursor).expect("Failed to read FSA5 with NUMBERS");

        assert_eq!(fsa.version, VERSION_FSA5);
        assert!(fsa.flags.contains(FsaFlags::NUMBERS));
        assert_eq!(fsa.goto_length, goto_len);
        assert_eq!(fsa.root_node_offset, root_offset_val);
        assert_eq!(fsa.get_root_node(), root_offset_val);

        let header_and_root_offset_len = 4 + 1 + 1 + 1 + 2 + (goto_len as usize);
        let expected_arcs_data = &fsa_file_bytes[header_and_root_offset_len..];
        assert_eq!(fsa.arcs_data, expected_arcs_data);
    }
    
    #[test]
    fn test_fsa5_from_reader_with_numbers_flag_gtl_zero() {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.push(VERSION_FSA5);
        data.push(0); 
        data.push(1); 
        data.extend_from_slice(&FsaFlags::NUMBERS.bits().to_le_bytes());

        let mut cursor = Cursor::new(data);
        let result = FSA5::from_reader(&mut cursor);
        assert_matches!(result, Err(FsaError::CorruptedFsa(_)));
    }


    #[test]
    fn test_read_address_helper() {
        assert_eq!(read_address(&[0x12], 1).unwrap(), 0x12);
        assert_eq!(read_address(&[0x12, 0x34], 2).unwrap(), 0x3412);
        assert_eq!(read_address(&[0x12, 0x34, 0x56], 3).unwrap(), 0x563412);
        assert_eq!(read_address(&[0x12, 0x34, 0x56, 0x78], 4).unwrap(), 0x78563412);
        assert_eq!(read_address(&[], 0).unwrap(), 0);
        assert_matches!(read_address(&[0x12], 2), Err(FsaError::CorruptedFsa(_)));
    }

    #[test]
    fn test_get_first_arc_on_empty_fsa_data_valid_root() {
        let mut fsa_file_bytes = Vec::new();
        fsa_file_bytes.extend_from_slice(&FSA_MAGIC);
        fsa_file_bytes.push(VERSION_FSA5);
        fsa_file_bytes.push(1);
        fsa_file_bytes.push(1);
        fsa_file_bytes.extend_from_slice(&FsaFlags::empty().bits().to_le_bytes());

        let mut cursor = Cursor::new(fsa_file_bytes);
        let fsa = FSA5::from_reader(&mut cursor).unwrap();
        assert_eq!(fsa.root_node_offset, 0);
        assert!(fsa.arcs_data.is_empty());

        let result = fsa.get_first_arc(fsa.get_root_node());
        assert_matches!(result, Err(FsaError::InvalidNodeOffset(0)));
    }

    #[test]
    fn test_get_arc_on_empty_fsa_data() {
        let mut fsa_file_bytes = Vec::new();
        fsa_file_bytes.extend_from_slice(&FSA_MAGIC);
        fsa_file_bytes.push(VERSION_FSA5);
        fsa_file_bytes.push(1); 
        fsa_file_bytes.push(1); 
        fsa_file_bytes.extend_from_slice(&FsaFlags::empty().bits().to_le_bytes());

        let mut cursor = Cursor::new(fsa_file_bytes);
        let fsa = FSA5::from_reader(&mut cursor).unwrap();
        assert_eq!(fsa.root_node_offset, 0);
        assert!(fsa.arcs_data.is_empty());

        let result = fsa.get_arc(fsa.get_root_node(), b'a');
        assert_matches!(result, Err(FsaError::ArcNotFound { label: b'a', node_offset: 0 }));
    }

}
