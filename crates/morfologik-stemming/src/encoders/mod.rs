// Moduł grupujący implementacje enkoderów sekwencji.

pub mod no_encoder;
pub mod trim_prefix_encoder;
pub mod trim_suffix_encoder;
pub mod trim_prefix_suffix_encoder;
pub mod trim_infix_suffix_encoder; 

// Re-eksportujemy, aby były łatwiej dostępne
pub use no_encoder::NoEncoder;
pub use trim_prefix_encoder::TrimPrefixEncoder;
pub use trim_suffix_encoder::TrimSuffixEncoder;
pub use trim_prefix_suffix_encoder::TrimPrefixAndSuffixEncoder; // Reeksportujemy
pub use trim_infix_suffix_encoder::TrimInfixAndSuffixEncoder;