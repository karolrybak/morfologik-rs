// Plik dla struktury WordData

use std::fmt;

/// Reprezentuje dane słowa: jego formę oryginalną, formę podstawową (stem) i tag.
/// Odpowiada `morfologik.stemming.WordData`.
///
/// Pola przechowują sekwencje bajtów, ponieważ słowniki Morfologika
/// operują na bajtach, a interpretacja kodowania znaków (np. na String)
/// może być wykonana później.
#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct WordData {
    /// Sekwencja bajtów reprezentująca słowo (forma fleksyjna).
    word_bytes: Vec<u8>,
    /// Sekwencja bajtów reprezentująca formę podstawową (stem).
    /// Może być `None`, jeśli stem nie jest dostępny lub jest taki sam jak słowo.
    stem_bytes: Option<Vec<u8>>,
    /// Sekwencja bajtów reprezentująca tag (informację gramatyczną).
    /// Może być `None`, jeśli tag nie jest dostępny.
    tag_bytes: Option<Vec<u8>>,
}

impl WordData {
    /// Tworzy nową instancję `WordData`.
    ///
    /// # Argumenty
    /// * `word` - Słowo (forma fleksyjna) jako sekwencja bajtów.
    /// * `stem` - Opcjonalna forma podstawowa (stem) jako sekwencja bajtów.
    /// * `tag` - Opcjonalny tag jako sekwencja bajtów.
    pub fn new(word: Vec<u8>, stem: Option<Vec<u8>>, tag: Option<Vec<u8>>) -> Self {
        WordData {
            word_bytes: word,
            stem_bytes: stem,
            tag_bytes: tag,
        }
    }

    /// Zwraca słowo (formę fleksyjną) jako plasterek bajtów.
    pub fn word(&self) -> &[u8] {
        &self.word_bytes
    }

    /// Zwraca formę podstawową (stem) jako opcjonalny plasterek bajtów.
    pub fn stem(&self) -> Option<&[u8]> {
        self.stem_bytes.as_deref()
    }

    /// Zwraca tag jako opcjonalny plasterek bajtów.
    pub fn tag(&self) -> Option<&[u8]> {
        self.tag_bytes.as_deref()
    }

    /// Ustawia słowo.
    pub fn set_word(&mut self, word: Vec<u8>) {
        self.word_bytes = word;
    }

    /// Ustawia formę podstawową (stem).
    pub fn set_stem(&mut self, stem: Option<Vec<u8>>) {
        self.stem_bytes = stem;
    }

    /// Ustawia tag.
    pub fn set_tag(&mut self, tag: Option<Vec<u8>>) {
        self.tag_bytes = tag;
    }

    // Metody do konwersji na String (z określonym kodowaniem) można dodać później, np.:
    // pub fn word_to_string(&self, encoding_name: &str) -> Result<String, YourEncodingError>
    // pub fn stem_to_string(&self, encoding_name: &str) -> Result<Option<String>, YourEncodingError>
    // pub fn tag_to_string(&self, encoding_name: &str) -> Result<Option<String>, YourEncodingError>
}

/// Implementacja `Debug` dla `WordData` próbująca zinterpretować bajty jako UTF-8.
/// Jeśli konwersja się nie uda, wyświetli surowe bajty.
impl fmt::Debug for WordData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Próba konwersji na String dla celów debugowania (zakładając UTF-8)
        let word_str = String::from_utf8_lossy(&self.word_bytes);
        let stem_str = self.stem_bytes.as_ref().map(|b| String::from_utf8_lossy(b));
        let tag_str = self.tag_bytes.as_ref().map(|b| String::from_utf8_lossy(b));

        f.debug_struct("WordData")
            .field("word", &word_str)
            .field("stem", &stem_str)
            .field("tag", &tag_str)
            .finish()
    }
}

/// Implementacja `Display` dla `WordData` próbująca zinterpretować bajty jako UTF-8.
/// Formatuje podobnie do `WordData.toString()` z Javy.
impl fmt::Display for WordData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WordData[{}, {}, {}]",
               String::from_utf8_lossy(&self.word_bytes),
               self.stem_bytes.as_ref().map_or_else(|| "null".into(), |b| String::from_utf8_lossy(b)),
               self.tag_bytes.as_ref().map_or_else(|| "null".into(), |b| String::from_utf8_lossy(b))
        )
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_data_creation_and_accessors() {
        let word = b"kocie".to_vec();
        let stem = Some(b"kot".to_vec());
        let tag = Some(b"N+sg+loc".to_vec());

        let wd = WordData::new(word.clone(), stem.clone(), tag.clone());

        assert_eq!(wd.word(), word.as_slice());
        assert_eq!(wd.stem(), stem.as_deref());
        assert_eq!(wd.tag(), tag.as_deref());
    }

    #[test]
    fn test_word_data_with_none_stem_tag() {
        let word = b"dom".to_vec();
        let wd = WordData::new(word.clone(), None, None);

        assert_eq!(wd.word(), word.as_slice());
        assert_eq!(wd.stem(), None);
        assert_eq!(wd.tag(), None);
    }

    #[test]
    fn test_word_data_setters() {
        let mut wd = WordData::default();

        let word1 = b"stare".to_vec();
        wd.set_word(word1.clone());
        assert_eq!(wd.word(), word1.as_slice());

        let stem1 = Some(b"stary".to_vec());
        wd.set_stem(stem1.clone());
        assert_eq!(wd.stem(), stem1.as_deref());

        let tag1 = Some(b"adj".to_vec());
        wd.set_tag(tag1.clone());
        assert_eq!(wd.tag(), tag1.as_deref());

        wd.set_stem(None);
        assert_eq!(wd.stem(), None);
    }

    #[test]
    fn test_word_data_debug_format() {
        let word = b"drzewo".to_vec(); // poprawne UTF-8
        let stem = Some(b"drzew".to_vec());
        let tag_invalid_utf8 = Some(vec![0xff, 0xfe, 0xfd]); // niepoprawne UTF-8

        let wd = WordData::new(word, stem, tag_invalid_utf8);
        let debug_str = format!("{:?}", wd);

        assert!(debug_str.contains("word: \"drzewo\""));
        assert!(debug_str.contains("stem: Some(\"drzew\")"));
        // Sprawdzenie, czy niepoprawny UTF-8 jest reprezentowany jakoś (np. przez )
        assert!(debug_str.contains("tag: Some(\"\u{fffd}\u{fffd}\u{fffd}\")"));
    }

    #[test]
    fn test_word_data_display_format() {
        let wd1 = WordData::new(b"kot".to_vec(), Some(b"kot".to_vec()), Some(b"N".to_vec()));
        assert_eq!(format!("{}", wd1), "WordData[kot, kot, N]");

        let wd2 = WordData::new(b"psy".to_vec(), None, Some(b"N+pl".to_vec()));
        assert_eq!(format!("{}", wd2), "WordData[psy, null, N+pl]");

        let wd3 = WordData::new(b"dom".to_vec(), Some(b"dom".to_vec()), None);
        assert_eq!(format!("{}", wd3), "WordData[dom, dom, null]");

        let wd4 = WordData::new(b"on".to_vec(), None, None);
        assert_eq!(format!("{}", wd4), "WordData[on, null, null]");
    }
}

