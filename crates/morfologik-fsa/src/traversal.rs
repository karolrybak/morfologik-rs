// Plik wygenerowany automatycznie przez skrypt.
// TODO: Dodać właściwą implementację.

use crate::fsa_trait::{ArcOffset, Fsa, Node}; 
use crate::error::{Result, FsaError}; 

/// Wynik dopasowania sekwencji w automacie FSA.
/// Odpowiada `morfologik.fsa.MatchResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchKind {
    ExactMatch,
    NoMatch,
    SequenceIsAPrefix,
    AutomatonIsAPrefix, 
}

/// Szczegółowy wynik operacji dopasowania sekwencji.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    pub kind: MatchKind,
    pub index: usize, 
    pub node: Node,   
}

/// Trait dla "odwiedzającego" stany podczas przechodzenia po FSA.
/// Odpowiada `morfologik.fsa.StateVisitor`.
pub trait StateVisitor<T: Fsa + ?Sized> { 
    fn visit_state(&mut self, fsa: &T, node: Node) -> bool;
    fn accept_arc(&mut self, fsa: &T, arc: ArcOffset) -> bool; 
}

/// Struktura pomocnicza do realizacji logiki z `FSATraversal.java`.
/// Udostępnia metody do dopasowywania sekwencji w danym automacie FSA.
pub struct FsaTraversal<'a, T: Fsa + ?Sized> { 
    fsa: &'a T,
}

impl<'a, T: Fsa + ?Sized> FsaTraversal<'a, T> { 
    pub fn new(fsa: &'a T) -> Self {
        FsaTraversal { fsa }
    }

    pub fn perfect_match(&self, sequence: &[u8]) -> Result<MatchResult> {
        let root_node = self.fsa.get_root_node();
        let mut current_node = root_node; 

        if sequence.is_empty() {
             match self.fsa.get_arc(current_node, 0) {
                Ok(final_arc) if self.fsa.is_arc_final(final_arc)? => {
                    return Ok(MatchResult { kind: MatchKind::ExactMatch, index: 0, node: current_node });
                }
                _ => return Ok(MatchResult { kind: MatchKind::NoMatch, index: 0, node: current_node }),
            }
        }

        let mut last_arc_taken: Option<ArcOffset> = None;
        let mut matched_len = 0;

        for (i, &byte) in sequence.iter().enumerate() {
            match self.fsa.get_arc(current_node, byte) {
                Ok(arc) => {
                    last_arc_taken = Some(arc);
                    current_node = self.fsa.get_end_node(arc)?;
                    matched_len = i + 1;
                }
                Err(FsaError::ArcNotFound { .. }) => {
                    return Ok(MatchResult { kind: MatchKind::NoMatch, index: matched_len, node: current_node });
                }
                Err(e) => return Err(e), 
            }
        }

        if let Some(final_arc) = last_arc_taken {
            if self.fsa.is_arc_final(final_arc)? {
                Ok(MatchResult { kind: MatchKind::ExactMatch, index: matched_len, node: current_node })
            } else {
                Ok(MatchResult { kind: MatchKind::SequenceIsAPrefix, index: matched_len, node: current_node })
            }
        } else {
            // Ten blok jest osiągany tylko jeśli `sequence` było puste i nie znaleziono łuku dla etykiety 0.
            // Wcześniejsza obsługa pustej sekwencji powinna to pokryć.
            Ok(MatchResult { kind: MatchKind::NoMatch, index: 0, node: root_node })
        }
    }

    pub fn visit_dfs<V: StateVisitor<T>>(
        &self,
        start_node: Node,
        path: &mut Vec<u8>,
        visitor: &mut V,
    ) -> Result<()> {
        // println!("[DFS ENTER] Node: {}, Path: {:?}", start_node, String::from_utf8_lossy(path));

        if !visitor.visit_state(self.fsa, start_node) {
            // println!("[DFS SKIP_NODE] Visitor returned false for node {}", start_node);
            return Ok(());
        }

        let mut current_arc_opt = match self.fsa.get_first_arc(start_node) {
            Ok(arc) => {
                // println!("[DFS GET_FIRST_ARC] Node: {}, First Arc: {}", start_node, arc);
                Some(arc)
            },
            Err(FsaError::InvalidNodeOffset(_)) => {
                // println!("[DFS GET_FIRST_ARC] Node: {}, No arcs or invalid node.", start_node);
                None
            }, 
            Err(e) => {
                // eprintln!("[DFS GET_FIRST_ARC] Node: {}, Error: {:?}", start_node, e);
                return Err(e);
            }
        };

        while let Some(arc) = current_arc_opt {
            // println!("[DFS LOOP_TOP] Node: {}, Processing Arc: {}", start_node, arc);
            
            let label = self.fsa.get_arc_label(arc)?; 
            let target_node = self.fsa.get_end_node(arc)?; 
            let is_terminal = self.fsa.is_arc_terminal(arc)?; 

            // println!("[DFS ARC_DETAILS] Arc: {}, Label: {}, Target: {}, IsTerminal: {}", arc, label as char, target_node, is_terminal);
            
            let should_traverse_deeper = visitor.accept_arc(self.fsa, arc);
            // if !should_traverse_deeper {
                // println!("[DFS SKIP_ARC_VISITOR] Visitor returned false for arc {}", arc);
            // }

            if should_traverse_deeper {
                path.push(label);
                // println!("[DFS RECURSE_PRE] To TargetNode: {}, NewPath: {:?}", target_node, String::from_utf8_lossy(path));
                self.visit_dfs(target_node, path, visitor)?;
                // println!("[DFS RECURSE_POST] From TargetNode: {}, Popping label, Path before pop: {:?}", target_node, String::from_utf8_lossy(path));
                path.pop(); 
                // println!("[DFS RECURSE_POST] Path after pop: {:?}", String::from_utf8_lossy(path));
            }

            if is_terminal {
                // println!("[DFS ARC_TERMINAL] Arc: {} is terminal. Ending loop for node {}.", arc, start_node);
                current_arc_opt = None; 
            } else {
                // println!("[DFS GET_NEXT_ARC] Arc: {} is not terminal. Getting next.", arc);
                match self.fsa.get_next_arc(arc) {
                    Ok(next_arc) => {
                        // println!("[DFS GET_NEXT_ARC] Next arc for {} is {}", arc, next_arc);
                        current_arc_opt = Some(next_arc);
                    }
                    Err(FsaError::NoNextArc(_)) => {
                        // println!("[DFS GET_NEXT_ARC] No next arc for {}. Ending loop for node {}.", arc, start_node);
                        current_arc_opt = None;
                    }
                    Err(e) => {
                        // eprintln!("[DFS GET_NEXT_ARC] Error getting next arc for {}: {:?}", arc, e);
                        return Err(e);
                    }
                }
            }
        }
        // println!("[DFS EXIT] Node: {}, Path: {:?}", start_node, String::from_utf8_lossy(path));
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsa5::FSA5;
    use crate::fsa5::{BIT_ARC_FINAL, BIT_ARC_LAST};
    use crate::fsa_header::{FSA_MAGIC, VERSION_FSA5};
    use std::io::Cursor;

    use assert_matches::assert_matches;

    fn create_linear_abc_fsa_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.push(VERSION_FSA5);
        data.push(1); 
        data.push(1); 
        data.extend_from_slice(&0u16.to_le_bytes()); 

        data.push(b'a');
        data.push(BIT_ARC_FINAL | BIT_ARC_LAST); 
        data.push(3);                            

        data.push(b'b');
        data.push(BIT_ARC_FINAL | BIT_ARC_LAST); 
        data.push(6);                            

        data.push(b'c');
        data.push(BIT_ARC_FINAL | BIT_ARC_LAST); 
        data.push(9);                            
        
        data
    }

    #[test]
    fn test_perfect_match_exact_linear() {
        let fsa_bytes = create_linear_abc_fsa_data();
        let fsa = FSA5::from_reader(&mut Cursor::new(fsa_bytes)).unwrap();
        let traversal = FsaTraversal::new(&fsa);

        let result_a = traversal.perfect_match(b"a").unwrap();
        assert_eq!(result_a.kind, MatchKind::ExactMatch);
        assert_eq!(result_a.index, 1);
        assert_eq!(result_a.node, 3);

        let result_ab = traversal.perfect_match(b"ab").unwrap();
        assert_eq!(result_ab.kind, MatchKind::ExactMatch);
        assert_eq!(result_ab.index, 2);
        assert_eq!(result_ab.node, 6);

        let result_abc = traversal.perfect_match(b"abc").unwrap();
        assert_eq!(result_abc.kind, MatchKind::ExactMatch);
        assert_eq!(result_abc.index, 3);
        assert_eq!(result_abc.node, 9);
    }

    #[test]
    fn test_perfect_match_no_match_linear() {
        let fsa_bytes = create_linear_abc_fsa_data();
        let fsa = FSA5::from_reader(&mut Cursor::new(fsa_bytes)).unwrap();
        let traversal = FsaTraversal::new(&fsa);

        let result_ax = traversal.perfect_match(b"ax").unwrap();
        assert_eq!(result_ax.kind, MatchKind::NoMatch, "Test 'ax'");
        assert_eq!(result_ax.index, 1); 
        assert_eq!(result_ax.node, 3);  

        let result_b = traversal.perfect_match(b"b").unwrap();
        assert_eq!(result_b.kind, MatchKind::NoMatch, "Test 'b'");
        assert_eq!(result_b.index, 0); 
        assert_eq!(result_b.node, fsa.get_root_node()); 

        let result_abd = traversal.perfect_match(b"abd").unwrap();
        assert_eq!(result_abd.kind, MatchKind::NoMatch, "Test 'abd'");
        assert_eq!(result_abd.index, 2); 
        assert_eq!(result_abd.node, 6);  
    }
    
    #[test]
    fn test_perfect_match_sequence_is_prefix_linear() {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC); data.push(VERSION_FSA5);
        data.push(1); data.push(1); data.extend_from_slice(&0u16.to_le_bytes());
        data.push(b'a'); data.push(BIT_ARC_FINAL | BIT_ARC_LAST); data.push(3);
        data.push(b'b'); data.push(BIT_ARC_LAST); data.push(6); 
        data.push(b'c'); data.push(BIT_ARC_FINAL | BIT_ARC_LAST); data.push(9);
        
        let fsa_bytes = data;
        let fsa = FSA5::from_reader(&mut Cursor::new(fsa_bytes)).unwrap();
        let traversal = FsaTraversal::new(&fsa);

        let result_a = traversal.perfect_match(b"a").unwrap();
        assert_eq!(result_a.kind, MatchKind::ExactMatch);

        let result_ab = traversal.perfect_match(b"ab").unwrap();
        assert_eq!(result_ab.kind, MatchKind::SequenceIsAPrefix);
        assert_eq!(result_ab.index, 2);
        assert_eq!(result_ab.node, 6);

        let result_abc = traversal.perfect_match(b"abc").unwrap();
        assert_eq!(result_abc.kind, MatchKind::ExactMatch);
    }


    struct TestVisitor {
        visited_nodes: Vec<Node>,
        accepted_arcs_labels: Vec<u8>,
        stop_at_node: Option<Node>,
        skip_arc_label: Option<u8>,
    }

    impl<F: Fsa + ?Sized> StateVisitor<F> for TestVisitor { 
        fn visit_state(&mut self, _fsa: &F, node: Node) -> bool {
            self.visited_nodes.push(node);
            if self.stop_at_node == Some(node) {
                return false;
            }
            true
        }

        fn accept_arc(&mut self, fsa: &F, arc: ArcOffset) -> bool { 
            let label = fsa.get_arc_label(arc).unwrap_or(b'?');
            if self.skip_arc_label == Some(label) {
                return false;
            }
            self.accepted_arcs_labels.push(label);
            true
        }
    }

    #[test]
    fn test_visit_dfs_full_traversal_linear() {
        let fsa_bytes = create_linear_abc_fsa_data();
        let fsa = FSA5::from_reader(&mut Cursor::new(fsa_bytes)).unwrap();
        let traversal = FsaTraversal::new(&fsa);

        let mut visitor = TestVisitor {
            visited_nodes: Vec::new(),
            accepted_arcs_labels: Vec::new(),
            stop_at_node: None,
            skip_arc_label: None,
        };
        let mut path = Vec::new();

        traversal.visit_dfs(fsa.get_root_node(), &mut path, &mut visitor).unwrap();

        assert_eq!(visitor.visited_nodes, vec![0, 3, 6, 9]); 
        assert_eq!(visitor.accepted_arcs_labels, vec![b'a', b'b', b'c']);
    }

    #[test]
    fn test_visit_dfs_stop_at_node_linear() {
        let fsa_bytes = create_linear_abc_fsa_data();
        let fsa = FSA5::from_reader(&mut Cursor::new(fsa_bytes)).unwrap();
        let traversal = FsaTraversal::new(&fsa);

        let mut visitor = TestVisitor {
            visited_nodes: Vec::new(),
            accepted_arcs_labels: Vec::new(),
            stop_at_node: Some(3), 
            skip_arc_label: None,
        };
        let mut path = Vec::new();
        traversal.visit_dfs(fsa.get_root_node(), &mut path, &mut visitor).unwrap();

        assert_eq!(visitor.visited_nodes, vec![0, 3]);
        assert_eq!(visitor.accepted_arcs_labels, vec![b'a']); 
    }

     #[test]
    fn test_visit_dfs_skip_arc_linear() {
        let fsa_bytes = create_linear_abc_fsa_data();
        let fsa = FSA5::from_reader(&mut Cursor::new(fsa_bytes)).unwrap();
        let traversal = FsaTraversal::new(&fsa);

        let mut visitor = TestVisitor {
            visited_nodes: Vec::new(),
            accepted_arcs_labels: Vec::new(),
            stop_at_node: None,
            skip_arc_label: Some(b'b'), 
        };
        let mut path = Vec::new();
        traversal.visit_dfs(fsa.get_root_node(), &mut path, &mut visitor).unwrap();
        
        assert_eq!(visitor.visited_nodes, vec![0, 3]); 
        assert_eq!(visitor.accepted_arcs_labels, vec![b'a']);
    }

    #[test]
    fn test_perfect_match_empty_sequence_on_abc_fsa_linear() {
        let fsa_bytes = create_linear_abc_fsa_data();
        let fsa = FSA5::from_reader(&mut Cursor::new(fsa_bytes)).unwrap();
        let traversal = FsaTraversal::new(&fsa);

        let result_empty = traversal.perfect_match(b"").unwrap();
        assert_matches!(result_empty.kind, MatchKind::NoMatch); 
        assert_eq!(result_empty.index, 0);
        assert_eq!(result_empty.node, fsa.get_root_node());
    }

    fn create_empty_and_a_fsa_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC);
        data.push(VERSION_FSA5);
        data.push(1);
        data.push(1);
        data.extend_from_slice(&0u16.to_le_bytes());

        data.push(0); 
        data.push(BIT_ARC_FINAL); 
        data.push(0); 

        data.push(b'a'); 
        data.push(BIT_ARC_FINAL | BIT_ARC_LAST); 
        data.push(0);
        data
    }

    #[test]
    fn test_perfect_match_empty_sequence_accepted() {
        let fsa_bytes = create_empty_and_a_fsa_data();
        let fsa = FSA5::from_reader(&mut Cursor::new(fsa_bytes)).unwrap();
        let traversal = FsaTraversal::new(&fsa);

        let result_empty = traversal.perfect_match(b"").unwrap();
        assert_eq!(result_empty.kind, MatchKind::ExactMatch);
        assert_eq!(result_empty.index, 0);
        assert_eq!(result_empty.node, fsa.get_root_node());

        let result_a = traversal.perfect_match(b"a").unwrap();
        assert_eq!(result_a.kind, MatchKind::ExactMatch);
        assert_eq!(result_a.index, 1);
        assert_eq!(result_a.node, 0); 
    }
}
