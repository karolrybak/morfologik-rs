// Plik wygenerowany automatycznie przez skrypt.
// TODO: Dodać właściwą implementację.

use std::fs::File;
use std::io::{self, BufReader, Cursor, Read, Seek};
use std::path::Path;

use crate::error::{FsaError, Result};
use crate::fsa_header::{FsaFlags, FsaHeader, VERSION_CFSA2};
use crate::fsa_trait::{ArcOffset, Fsa, Node};

const CFSA_ARC_IS_FINAL_BIT: u8 = 1 << 0; 
const CFSA_ARC_IS_LAST_BIT: u8 = 1 << 1;  
const CFSA_ARC_TARGET_IS_NEXT_BIT: u8 = 1 << 2; 

const CFSA2_LABEL_MASK: u8 = 0xf8; 
const CFSA2_LABEL_SHIFT: u8 = 3;
const CFSA2_FLAGS_MASK: u8 = 0x07; 


#[derive(Debug)]
pub struct CFSA2 {
    pub version: u8,
    pub flags: FsaFlags,
    pub goto_length_info: u8,
    pub arcs_data: Vec<u8>,
    pub root_node_offset: usize,
}

impl CFSA2 {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).map_err(FsaError::Io)?;
        let mut reader = BufReader::new(file);
        Self::from_reader(&mut reader)
    }

    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let header = FsaHeader::read(reader)?;
        if header.version != VERSION_CFSA2 {
            return Err(FsaError::UnsupportedVersion(header.version));
        }

        let mut file_flags_buffer = [0u8; 2]; 
        reader.read_exact(&mut file_flags_buffer).map_err(map_io_error)?;
        
        let automaton_flags_short = u16::from_le_bytes(file_flags_buffer);
        let flags = FsaFlags::from_bits_truncate(automaton_flags_short & 0x00FF); 
        let goto_length_info = ((automaton_flags_short & 0xFF00) >> 8) as u8; 

        let root_node_offset = if flags.contains(FsaFlags::NUMBERS) {
            let (offset, _bytes_read) = read_vint(reader)?;
            offset
        } else {
            0 
        };

        let mut arcs_data = Vec::new();
        reader.read_to_end(&mut arcs_data).map_err(map_io_error)?;

        Ok(CFSA2 {
            version: header.version,
            flags,
            goto_length_info,
            arcs_data,
            root_node_offset,
        })
    }

    #[inline]
    fn read_vint_at(&self, offset: usize) -> Result<(usize, usize)> {
        if offset >= self.arcs_data.len() {
            if self.arcs_data.is_empty() && offset == 0 { 
                return Err(FsaError::CorruptedFsa(format!(
                    "VInt read offset 0 on empty arcs_data"
                )));
            }
            return Err(FsaError::CorruptedFsa(format!(
                "VInt read offset {} out of bounds (len: {})",
                offset, self.arcs_data.len()
            )));
        }
        let mut cursor = Cursor::new(&self.arcs_data[offset..]);
        read_vint(&mut cursor)
    }
}

/// Pomocnicza funkcja do odczytu VInt z Readera.
/// Zwraca (wartość, liczba odczytanych bajtów).
fn read_vint<R: Read>(reader: &mut R) -> Result<(usize, usize)> {
    let mut result: usize = 0;
    let mut shift: u32 = 0; // Zmieniono typ na u32, aby uniknąć przepełnienia przy shift > 63
    let mut bytes_read: usize = 0;
    let mut current_byte_val: u8;

    loop {
        if bytes_read >= 10 { // Max 10 bytes for up to 70 bits, well within usize on 64-bit
            return Err(FsaError::CorruptedFsa(format!("VInt too long (read {} bytes)", bytes_read)));
        }
        
        let mut buffer = [0u8; 1];
        reader.read_exact(&mut buffer).map_err(map_io_error)?;
        current_byte_val = buffer[0];
        bytes_read += 1;

        // Dodaj 7 bitów wartości do wyniku
        result |= ((current_byte_val & 0x7F) as usize) << shift;
        
        if (current_byte_val & 0x80) == 0 { // MSB = 0 oznacza ostatni bajt
            break;
        }
        shift += 7; // Przygotuj shift dla następnych 7 bitów
    }
    Ok((result, bytes_read))
}


fn map_io_error(e: std::io::Error) -> FsaError {
    if e.kind() == io::ErrorKind::UnexpectedEof {
        FsaError::UnexpectedEof
    } else {
        FsaError::Io(e)
    }
}


impl Fsa for CFSA2 {
    fn get_flags(&self) -> FsaFlags {
        self.flags
    }

    fn get_root_node(&self) -> Node {
        self.root_node_offset
    }

    fn get_first_arc(&self, node_offset: Node) -> Result<ArcOffset> {
        if node_offset >= self.arcs_data.len() {
            if self.arcs_data.is_empty() && node_offset == 0 {
                 return Err(FsaError::InvalidNodeOffset(node_offset));
            }
            return Err(FsaError::InvalidNodeOffset(node_offset));
        }
        Ok(node_offset)
    }

    fn get_next_arc(&self, current_arc_offset: ArcOffset) -> Result<ArcOffset> {
        if current_arc_offset >= self.arcs_data.len() {
            return Err(FsaError::InvalidArcOffset(current_arc_offset));
        }

        let label_and_flags = self.arcs_data[current_arc_offset];
        let arc_flags = label_and_flags & CFSA2_FLAGS_MASK;

        if (arc_flags & CFSA_ARC_IS_LAST_BIT) != 0 {
            return Err(FsaError::NoNextArc(current_arc_offset));
        }

        let (_target_addr, target_addr_bytes) = self.read_vint_at(
            current_arc_offset.checked_add(1)
            .ok_or_else(|| FsaError::CorruptedFsa(format!("Offset overflow for VInt read at arc {}", current_arc_offset)))?
        )?;
        
        let next_arc_offset = current_arc_offset
            .checked_add(1) 
            .ok_or_else(|| FsaError::CorruptedFsa(format!("Offset overflow calculating next arc (step 1) for arc {}", current_arc_offset)))?
            .checked_add(target_addr_bytes) 
            .ok_or_else(|| FsaError::CorruptedFsa(format!("Offset overflow calculating next arc (step 2) for arc {}", current_arc_offset)))?;

        if next_arc_offset >= self.arcs_data.len() {
            return Err(FsaError::CorruptedFsa(format!(
                "Calculated next arc offset {} points beyond or at end of data (len: {}) for arc {}",
                next_arc_offset, self.arcs_data.len(), current_arc_offset
            )));
        }
        Ok(next_arc_offset)
    }

    fn get_arc(&self, node_offset: Node, label: u8) -> Result<ArcOffset> {
        if node_offset >= self.arcs_data.len() && !(self.arcs_data.is_empty() && node_offset == 0) {
            return Err(FsaError::InvalidNodeOffset(node_offset));
        }
        if self.arcs_data.is_empty() && node_offset == 0 { 
             return Err(FsaError::ArcNotFound { label, node_offset });
        }

        let mut current_arc_res = self.get_first_arc(node_offset);

        while let Ok(arc_offset) = current_arc_res {
            if arc_offset >= self.arcs_data.len() {
                return Err(FsaError::CorruptedFsa(format!("Arc offset {} out of bounds while searching for label {}", arc_offset, label)));
            }
            
            let label_and_flags = self.arcs_data[arc_offset];
            let arc_label = (label_and_flags & CFSA2_LABEL_MASK) >> CFSA2_LABEL_SHIFT;
            let arc_flags = label_and_flags & CFSA2_FLAGS_MASK;

            if arc_label == label {
                return Ok(arc_offset);
            }

            if (arc_flags & CFSA_ARC_IS_LAST_BIT) != 0 {
                break; 
            }
            current_arc_res = self.get_next_arc(arc_offset);
        }
        Err(FsaError::ArcNotFound { label, node_offset })
    }

    fn get_end_node(&self, arc_offset: ArcOffset) -> Result<Node> {
        if arc_offset.checked_add(1).map_or(true, |vint_start| vint_start > self.arcs_data.len()) {
             return Err(FsaError::InvalidArcOffset(arc_offset));
        }
        if arc_offset >= self.arcs_data.len() {
            return Err(FsaError::InvalidArcOffset(arc_offset));
        }

        let label_and_flags = self.arcs_data[arc_offset];
        let arc_flags = label_and_flags & CFSA2_FLAGS_MASK;

        let (target_address_val, bytes_read_for_vint) = self.read_vint_at(arc_offset + 1)?;

        if (arc_flags & CFSA_ARC_TARGET_IS_NEXT_BIT) != 0 {
            Ok(target_address_val)
        } else {
            let offset_after_vint = arc_offset
                .checked_add(1) 
                .ok_or_else(|| FsaError::CorruptedFsa(format!("End node offset overflow (step 1) for arc {}", arc_offset)))?
                .checked_add(bytes_read_for_vint) 
                .ok_or_else(|| FsaError::CorruptedFsa(format!("End node offset overflow (step 2) for arc {}", arc_offset)))?;
            
            offset_after_vint.checked_add(target_address_val)
                .ok_or_else(|| FsaError::CorruptedFsa(format!("End node offset overflow (step 3) for arc {}", arc_offset)))
        }
    }

    fn get_arc_label(&self, arc_offset: ArcOffset) -> Result<u8> {
        if arc_offset < self.arcs_data.len() {
            let label_and_flags = self.arcs_data[arc_offset];
            Ok((label_and_flags & CFSA2_LABEL_MASK) >> CFSA2_LABEL_SHIFT)
        } else {
            Err(FsaError::InvalidArcOffset(arc_offset))
        }
    }

    fn is_arc_final(&self, arc_offset: ArcOffset) -> Result<bool> {
        if arc_offset < self.arcs_data.len() {
            let label_and_flags = self.arcs_data[arc_offset];
            Ok((label_and_flags & CFSA_ARC_IS_FINAL_BIT) != 0)
        } else {
            Err(FsaError::InvalidArcOffset(arc_offset))
        }
    }

    fn is_arc_terminal(&self, arc_offset: ArcOffset) -> Result<bool> {
        if arc_offset < self.arcs_data.len() {
            let label_and_flags = self.arcs_data[arc_offset];
            Ok((label_and_flags & CFSA_ARC_IS_LAST_BIT) != 0)
        } else {
            Err(FsaError::InvalidArcOffset(arc_offset))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsa_header::FSA_MAGIC;
    use std::io::Cursor;
    use assert_matches::assert_matches;

    /// Poprawiona funkcja do konwersji liczby na VInt (sekwencję bajtów).
    /// Zapisuje najmniej znaczące 7 bitów jako pierwsze.
    fn vint_to_bytes(mut value: usize) -> Vec<u8> {
        if value == 0 {
            return vec![0];
        }
        let mut result = Vec::new();
        loop {
            let byte = (value & 0x7F) as u8; // Weź 7 najniższych bitów
            value >>= 7;                         // Przesuń o 7 bitów w prawo
            if value == 0 {
                result.push(byte);               // Ostatni bajt, MSB=0
                break;
            } else {
                result.push(byte | 0x80);        // Są kolejne bajty, ustaw MSB=1
            }
        }
        result 
    }
    
    #[test]
    fn test_vint_conversion_and_read() {
        // Testy dla vint_to_bytes
        assert_eq!(vint_to_bytes(0), vec![0x00]);       // 0
        assert_eq!(vint_to_bytes(1), vec![0x01]);       // 1
        assert_eq!(vint_to_bytes(127), vec![0x7F]);     // 127 (0b01111111)
        assert_eq!(vint_to_bytes(128), vec![0x80, 0x01]); // 128 (0b10000000) -> 0x80 (0) | 0x01 (1 << 7)
        assert_eq!(vint_to_bytes(300), vec![0xAC, 0x02]); // 300 = 44 + (2 << 7) = 44 + 256. 300&0x7F=44 (0x2C). 300>>7 = 2.
                                                       // B0: (300 & 0x7F) | 0x80 = 0x2C | 0x80 = 0xAC
                                                       // B1: (2 & 0x7F) = 0x02
        assert_eq!(vint_to_bytes(0x3FFF), vec![0xFF, 0x7F]); // 16383
                                                            // B0: (16383 & 0x7F) | 0x80 = 0x7F | 0x80 = 0xFF
                                                            // 16383 >> 7 = 127 (0x7F)
                                                            // B1: (127 & 0x7F) = 0x7F


        // Testy dla read_vint
        let mut cursor_0 = Cursor::new(vint_to_bytes(0));
        assert_eq!(read_vint(&mut cursor_0).unwrap(), (0, 1));
        let mut cursor_1 = Cursor::new(vint_to_bytes(1));
        assert_eq!(read_vint(&mut cursor_1).unwrap(), (1, 1));
        let mut cursor_127 = Cursor::new(vint_to_bytes(127));
        assert_eq!(read_vint(&mut cursor_127).unwrap(), (127, 1));
        let mut cursor_128 = Cursor::new(vint_to_bytes(128));
        assert_eq!(read_vint(&mut cursor_128).unwrap(), (128, 2));
        let mut cursor_300 = Cursor::new(vint_to_bytes(300));
        assert_eq!(read_vint(&mut cursor_300).unwrap(), (300, 2));
        let mut cursor_max_2_byte = Cursor::new(vint_to_bytes(0x3FFF));
        assert_eq!(read_vint(&mut cursor_max_2_byte).unwrap(), (0x3FFF, 2));


        // Test VInt too long
        let too_long_vint_data = vec![0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01]; // 11 bajtów
        let mut cursor_long = Cursor::new(too_long_vint_data);
        assert_matches!(read_vint(&mut cursor_long), Err(FsaError::CorruptedFsa(_)));
    }


    fn create_cfsa2_ab_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.push(VERSION_CFSA2);
        let gtl_info = 0u8; 
        let automaton_flags_short: u16 = (FsaFlags::empty().bits() & 0x00FF) | ((gtl_info & 0xFF) as u16) << 8;
        data.extend_from_slice(&automaton_flags_short.to_le_bytes());

        let label_a = 1u8; 
        data.push((label_a << CFSA2_LABEL_SHIFT) | CFSA_ARC_IS_FINAL_BIT); 
        data.extend_from_slice(&vint_to_bytes(0)); 

        let label_b = 2u8; 
        data.push((label_b << CFSA2_LABEL_SHIFT) | CFSA_ARC_IS_FINAL_BIT | CFSA_ARC_IS_LAST_BIT); 
        data.extend_from_slice(&vint_to_bytes(0)); 
        data
    }
    

    #[test]
    fn test_cfsa2_from_reader_simple_ab() {
        let fsa_data = create_cfsa2_ab_data();
        let mut cursor = Cursor::new(fsa_data.clone());
        let fsa = CFSA2::from_reader(&mut cursor).expect("Failed to read CFSA2");

        assert_eq!(fsa.version, VERSION_CFSA2);
        assert_eq!(fsa.flags, FsaFlags::empty());
        assert_eq!(fsa.goto_length_info, 0); 
        assert_eq!(fsa.root_node_offset, 0);

        let header_len = 4 + 1 + 2; 
        let expected_arcs_data = &fsa_data[header_len..];
        assert_eq!(fsa.arcs_data, expected_arcs_data);
    }

    #[test]
    fn test_cfsa2_arc_access() {
        let fsa_data = create_cfsa2_ab_data();
        let mut cursor = Cursor::new(fsa_data);
        let fsa = CFSA2::from_reader(&mut cursor).unwrap();
        let root = fsa.get_root_node(); 

        let arc_a = fsa.get_arc(root, 1).unwrap(); 
        assert_eq!(arc_a, 0); 
        assert_eq!(fsa.get_arc_label(arc_a).unwrap(), 1);
        assert!(fsa.is_arc_final(arc_a).unwrap());
        assert!(!fsa.is_arc_terminal(arc_a).unwrap()); 
        assert_eq!(fsa.get_end_node(arc_a).unwrap(), 2);

        let arc_b = fsa.get_next_arc(arc_a).unwrap();
        assert_eq!(arc_b, 2); 
        assert_eq!(fsa.get_arc_label(arc_b).unwrap(), 2);
        assert!(fsa.is_arc_final(arc_b).unwrap());
        assert!(fsa.is_arc_terminal(arc_b).unwrap()); 
        assert_eq!(fsa.get_end_node(arc_b).unwrap(), 4);


        let arc_b_direct = fsa.get_arc(root, 2).unwrap();
        assert_eq!(arc_b_direct, 2);

        assert_matches!(fsa.get_next_arc(arc_b), Err(FsaError::NoNextArc(2)));
        assert_matches!(fsa.get_arc(root, 3), Err(FsaError::ArcNotFound { label: 3, node_offset: 0 }));
    }

    #[test]
    fn test_cfsa2_get_end_node_target_is_next() {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.push(VERSION_CFSA2);
        let gtl_info = 0u8;
        let automaton_flags_short: u16 = (FsaFlags::empty().bits() & 0x00FF) | ((gtl_info & 0xFF) as u16) << 8;
        data.extend_from_slice(&automaton_flags_short.to_le_bytes());

        let label = 0u8;
        let target_node_addr = 218; 
        data.push((label << CFSA2_LABEL_SHIFT) | CFSA_ARC_TARGET_IS_NEXT_BIT | CFSA_ARC_IS_LAST_BIT);
        data.extend_from_slice(&vint_to_bytes(target_node_addr)); 
        
        let mut cursor = Cursor::new(data);
        let fsa = CFSA2::from_reader(&mut cursor).unwrap();
        let root = fsa.get_root_node();
        let arc = fsa.get_first_arc(root).unwrap();

        assert_eq!(fsa.get_end_node(arc).unwrap(), target_node_addr);
    }
}

