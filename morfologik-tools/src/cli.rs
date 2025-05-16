// Definicje argumentów CLI (np. z clap)
// use clap::{Parser, Subcommand};

// #[derive(Parser, Debug)]
// #[clap(author, version, about, long_about = None)]
// pub struct Cli {
//     #[clap(subcommand)]
//     pub command: Commands,
// }

// #[derive(Subcommand, Debug)]
// pub enum Commands {
//     /// Buduje automat FSA z pliku tekstowego
//     FsaBuild {
//         #[clap(short, long, value_parser)]
//         input: std::path::PathBuf,
//         #[clap(short, long, value_parser)]
//         output: std::path::PathBuf,
//     },
//     // Dodaj inne komendy
// }
