// Plik wygenerowany automatycznie przez skrypt.
// TODO: Dodać właściwą implementację.

use crate::error::Result; 
use crate::fsa_header::FsaFlags; 
use std::fmt::Debug; // Import Debug

/// Reprezentuje węzeł w automacie FSA.
pub type Node = usize;

/// Reprezentuje offset łuku (przejścia) w automacie FSA.
pub type ArcOffset = usize;

/// Ogólny trait dla automatów skończonych (FSA).
/// Wymaga implementacji Debug, aby struktury zawierające Box<dyn Fsa> mogły derive(Debug).
pub trait Fsa: Debug + Send + Sync { // Dodano Debug jako supertrait
    fn get_flags(&self) -> FsaFlags;
    fn get_root_node(&self) -> Node;
    fn get_first_arc(&self, node: Node) -> Result<ArcOffset>;
    fn get_next_arc(&self, arc: ArcOffset) -> Result<ArcOffset>;
    fn get_arc(&self, node: Node, label: u8) -> Result<ArcOffset>;
    fn get_end_node(&self, arc: ArcOffset) -> Result<Node>;
    fn get_arc_label(&self, arc: ArcOffset) -> Result<u8>;
    fn is_arc_final(&self, arc: ArcOffset) -> Result<bool>;
    fn is_arc_terminal(&self, arc: ArcOffset) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
