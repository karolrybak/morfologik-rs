// Plik wygenerowany automatycznie przez skrypt.
// TODO: Dodać właściwą implementację.

use crate::fsa_trait::{ArcOffset, Fsa, Node}; 
use crate::error::{Result as FsaResult, FsaError}; 

/// Iterator po sekwencjach bajtów (słowach) w automacie FSA.
#[derive(Debug)]
pub struct ByteSequenceIterator<'a, T: Fsa + ?Sized + 'a> {
    fsa: &'a T,
    // Stack stores: (node_id, option_of_next_arc_offset_from_this_node_to_try)
    stack: Vec<(Node, Option<ArcOffset>)>, 
    current_sequence: Vec<u8>,
    #[allow(dead_code)]
    start_node: Option<Node>, // For potential future use if iterator needs to restart from a specific sequence
}

impl<'a, T: Fsa + ?Sized> ByteSequenceIterator<'a, T> {
    pub fn new(fsa: &'a T) -> Self {
        Self::from_node(fsa, fsa.get_root_node())
    }

    pub fn from_node(fsa: &'a T, start_node_id: Node) -> Self {
        let mut iterator = ByteSequenceIterator {
            fsa,
            stack: Vec::new(),
            current_sequence: Vec::new(),
            start_node: Some(start_node_id),
        };
        
        // Initialize stack with the start node and its first arc (if any)
        match fsa.get_first_arc(start_node_id) {
            Ok(first_arc) => {
                iterator.stack.push((start_node_id, Some(first_arc)));
            }
            Err(_) => {
                // If start_node_id is invalid or has no arcs, stack remains empty,
                // and the iterator will correctly produce no items.
            }
        }
        iterator
    }

    fn find_next_sequence(&mut self) -> Option<FsaResult<Vec<u8>>> {
        loop {
            // Peek at the top of the stack to see which node and arc to process.
            // We use `last_mut` to be able to update the `Option<ArcOffset>` (next sibling).
            let (current_node_on_stack, next_arc_to_try_ref) = match self.stack.last_mut() {
                Some(top) => (top.0, &mut top.1), // top.0 is Node, top.1 is Option<ArcOffset>
                None => return None, // Stack is empty, DFS is complete.
            };

            let arc_to_process_opt = next_arc_to_try_ref.take(); // Take the arc, leaving None for this node's turn

            if let Some(arc) = arc_to_process_opt {
                // We are processing 'arc' from 'current_node_on_stack'.
                // Before going deeper, put its *next sibling* back onto the stack for later.
                if let Ok(false) = self.fsa.is_arc_terminal(arc) { // If current arc is not the last from its node
                    if let Ok(next_sibling_arc) = self.fsa.get_next_arc(arc) {
                        // This assignment is to the stack frame we are currently looking at.
                        // When this arc's processing (and its children's) is done,
                        // the loop will pick up this next_sibling_arc.
                        *next_arc_to_try_ref = Some(next_sibling_arc);
                    }
                    // If get_next_arc fails (e.g. NoNextArc, which shouldn't happen if not terminal),
                    // or if is_arc_terminal errored, then None remains, and this node's siblings are done.
                } // If it was terminal, None correctly remains.

                let label = match self.fsa.get_arc_label(arc) {
                    Ok(l) => l,
                    Err(e) => {
                        // Error getting label, current path is corrupted.
                        // Pop the current_sequence label that was *about* to be added or was from parent.
                        // This state is tricky; best to return error.
                        // The stack will be popped in the next iteration if this was the only arc.
                        return Some(Err(e));
                    }
                };
                self.current_sequence.push(label);

                let is_final = match self.fsa.is_arc_final(arc) {
                    Ok(f) => f,
                    Err(e) => { self.current_sequence.pop(); return Some(Err(e)); }
                };

                let mut sequence_yielded_for_this_arc: Option<FsaResult<Vec<u8>>> = None;
                if is_final {
                    sequence_yielded_for_this_arc = Some(Ok(self.current_sequence.clone()));
                }

                // Try to go deeper: push children of the current arc's target node to stack.
                let mut pushed_children = false;
                match self.fsa.get_end_node(arc) {
                    Ok(target_node) => {
                        match self.fsa.get_first_arc(target_node) {
                            Ok(first_child_arc) => {
                                self.stack.push((target_node, Some(first_child_arc)));
                                pushed_children = true;
                            }
                            Err(FsaError::InvalidNodeOffset(_)) => { /* Target is a leaf, no children to push */ }
                            Err(e) => { // Error getting children
                                self.current_sequence.pop(); // Backtrack label
                                return sequence_yielded_for_this_arc.or(Some(Err(e)));
                            }
                        }
                    }
                    Err(e) => { // Error getting end node
                        self.current_sequence.pop(); // Backtrack label
                        return sequence_yielded_for_this_arc.or(Some(Err(e)));
                    }
                }

                if sequence_yielded_for_this_arc.is_some() {
                    // If we yielded a sequence, and we are *not* going deeper from this arc's target,
                    // we need to pop the current label because we are done with this path.
                    // If we *are* going deeper, the pop will happen when that deeper path is exhausted.
                    if !pushed_children {
                        self.current_sequence.pop();
                    }
                    return sequence_yielded_for_this_arc;
                }

                // If not final, and we didn't push children (it's a non-final leaf for this path)
                if !is_final && !pushed_children {
                    self.current_sequence.pop(); // Backtrack this label
                }
                // Continue the loop: if children were pushed, they'll be processed.
                // Otherwise, the (now None) arc option for current_node_on_stack will cause a pop.

            } else {
                // No more arcs to try from current_node_on_stack (next_arc_to_try_opt_mut was None).
                // This means we have processed all children of this node. Time to backtrack.
                self.stack.pop(); // Pop (current_node_on_stack, None)
                if !self.current_sequence.is_empty() {
                    // Pop the label that led to current_node_on_stack
                    self.current_sequence.pop();
                }
            }
        }
        None // Stack is empty
    }
}

impl<'a, T: Fsa + ?Sized> Iterator for ByteSequenceIterator<'a, T> {
    type Item = FsaResult<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.find_next_sequence()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsa5::FSA5; 
    use crate::fsa_header::{FSA_MAGIC, VERSION_FSA5};
    use crate::fsa5::{BIT_ARC_FINAL, BIT_ARC_LAST}; 
    use std::io::Cursor;
    use std::collections::HashSet;
    use std::sync::Arc;

    /// Creates FSA for: "a", "ab", "abc", "ax"
    /// Structure:
    /// Node 0 (root, offset 0):
    ///   Arc 'a' (FINAL) -> target Node 1 (offset 3) [LAST arc from Node 0]
    /// Node 1 (state after "a", offset 3):
    ///   Arc 'b' (FINAL) -> target Node 2 (offset 9) [NOT LAST from Node 1]
    ///   Arc 'x' (FINAL) -> target dummy_x_target (101) [LAST arc from Node 1]
    /// Node 2 (state after "ab", offset 9):
    ///   Arc 'c' (FINAL) -> target dummy_c_target (100) [LAST arc from Node 2]
    fn create_test_fsa_for_iterator() -> FSA5 {
        let mut arcs_data = Vec::new();
        let dummy_c_target = 100; 
        let dummy_x_target = 101;

        // Arcs from Node 0 (root, offset 0 in arcs_data)
        // Arc 'a'
        arcs_data.push(b'a');
        arcs_data.push(BIT_ARC_FINAL | BIT_ARC_LAST); // "a" is a word. It's the only arc from Node 0.
        arcs_data.push(3);                           // Target: Node 1 (offset 3)

        // Arcs from Node 1 (state after "a", starts at offset 3 in arcs_data)
        // Arc 'b'
        arcs_data.push(b'b');
        arcs_data.push(BIT_ARC_FINAL);              // "ab" is a word. NOT LAST from Node 1 (because 'x' follows).
        arcs_data.push(9);                          // Target: Node 2 (offset 9)
        // Arc 'x'
        arcs_data.push(b'x');
        arcs_data.push(BIT_ARC_FINAL | BIT_ARC_LAST); // "ax" is a word. IS LAST from Node 1.
        arcs_data.push(dummy_x_target);

        // Arcs from Node 2 (state after "ab", starts at offset 9 in arcs_data)
        // Arc 'c'
        arcs_data.push(b'c');
        arcs_data.push(BIT_ARC_FINAL | BIT_ARC_LAST); // "abc" is a word. IS LAST from Node 2.
        arcs_data.push(dummy_c_target);

        let mut header_data = Vec::new();
        header_data.extend_from_slice(&FSA_MAGIC);
        header_data.push(VERSION_FSA5);
        header_data.push(1); 
        header_data.push(1); 
        header_data.extend_from_slice(&0u16.to_le_bytes()); 

        let mut fsa_file_bytes = header_data;
        fsa_file_bytes.extend_from_slice(&arcs_data);
        
        FSA5::from_reader(&mut Cursor::new(fsa_file_bytes)).unwrap()
    }

    #[test]
    fn test_byte_sequence_iterator_simple() {
        let fsa = create_test_fsa_for_iterator();
        let iterator = ByteSequenceIterator::new(&fsa);

        let mut results = HashSet::new();
        for seq_res in iterator {
            match seq_res {
                Ok(seq) => { results.insert(seq); }
                Err(e) => panic!("Iterator returned an error: {:?}", e),
            }
        }

        let expected_sequences: HashSet<Vec<u8>> = [
            b"a".to_vec(), b"ab".to_vec(), b"abc".to_vec(), b"ax".to_vec(),
        ].iter().cloned().collect();
        
        assert_eq!(results, expected_sequences, "Test: simple iterator results mismatch");
    }

    #[test]
    fn test_iterator_with_dyn_fsa() {
        let fsa_concrete = create_test_fsa_for_iterator();
        let fsa_dyn: Arc<Box<dyn Fsa + Send + Sync>> = Arc::new(Box::new(fsa_concrete));
        
        let iterator = ByteSequenceIterator::new(fsa_dyn.as_ref().as_ref());

        let mut results = HashSet::new();
        for seq_res in iterator { results.insert(seq_res.unwrap()); }
        let expected_sequences: HashSet<Vec<u8>> = [
            b"a".to_vec(), b"ab".to_vec(), b"abc".to_vec(), b"ax".to_vec(),
        ].iter().cloned().collect();
        assert_eq!(results, expected_sequences, "Test: dyn fsa iterator results mismatch");
    }


    #[test]
    fn test_iterator_on_fsa_with_only_root_no_arcs() {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC); data.push(VERSION_FSA5);
        data.push(1); data.push(1); data.extend_from_slice(&0u16.to_le_bytes());

        let fsa = FSA5::from_reader(&mut Cursor::new(data)).unwrap();
        let mut iterator = ByteSequenceIterator::new(&fsa); 
        assert!(iterator.next().is_none()); 
    }

    #[test]
    fn test_iterator_on_fsa_single_node_final_empty_seq() {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC); data.push(VERSION_FSA5);
        data.push(1); data.push(1); data.extend_from_slice(&0u16.to_le_bytes());
        data.push(0); 
        data.push(BIT_ARC_FINAL | BIT_ARC_LAST); 
        data.push(100); 

        let fsa = FSA5::from_reader(&mut Cursor::new(data)).unwrap();
        let iterator = ByteSequenceIterator::new(&fsa);
        let results: Vec<_> = iterator.map(|r| r.unwrap()).collect();
        
        let expected: Vec<Vec<u8>> = vec![vec![0u8]]; 
        assert_eq!(results, expected);
    }
    
    /// Creates FSA for "a" (final) and "b" (final) as separate branches from the root.
    /// Node 0 (root):
    ///   Arc 'a' (FINAL) -> dummy_target_a [NOT LAST]
    ///   Arc 'b' (FINAL) -> dummy_target_b [LAST]
    fn create_fsa_a_b_separate() -> FSA5 {
        let mut data = Vec::new();
        data.extend_from_slice(&FSA_MAGIC); data.push(VERSION_FSA5);
        data.push(1); data.push(1); data.extend_from_slice(&0u16.to_le_bytes());
        
        let dummy_target_a = 100;
        let dummy_target_b = 101;

        // Arc 'a' (offset 0) from root. Not last, 'b' follows from root.
        data.push(b'a'); 
        data.push(BIT_ARC_FINAL); // NOT LAST
        data.push(dummy_target_a); 
        
        // Arc 'b' (offset 3) from root. Last from root.
        data.push(b'b'); 
        data.push(BIT_ARC_FINAL | BIT_ARC_LAST); // IS LAST
        data.push(dummy_target_b); 

        FSA5::from_reader(&mut Cursor::new(data)).unwrap()
    }

    #[test]
    fn test_iterator_a_b_separate() {
        let fsa = create_fsa_a_b_separate();
        let iterator = ByteSequenceIterator::new(&fsa);
        let results: HashSet<Vec<u8>> = iterator.map(|r| r.unwrap()).collect();
        let expected: HashSet<Vec<u8>> = [b"a".to_vec(), b"b".to_vec()].iter().cloned().collect();
        assert_eq!(results, expected, "Test: a_b_separate iterator results mismatch");
    }
}
