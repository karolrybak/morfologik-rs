pub mod error;
pub mod fsa_header;
pub mod fsa_trait;
pub mod fsa5;
pub mod cfsa2;
pub mod traversal;
pub mod iterator; // Dodajemy nowy moduł

// Przykładowa funkcja
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
