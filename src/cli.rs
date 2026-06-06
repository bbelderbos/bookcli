use std::io::Write;

use clap::{Parser, Subcommand};

use crate::error::Result;
use crate::search::BookSearch;

#[derive(Parser)]
#[command(name = "book", version, about = "Track your reading from the terminal")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Search Google Books for a query
    Search { query: String },
}

// Handler takes the search seam + a writer so it's unit-testable with a
// FakeSearch and a `Vec<u8>` buffer — no network, no stdout.
pub fn run_search(search: &impl BookSearch, query: &str, out: &mut impl Write) -> Result<()> {
    let hits = search.search(query)?;
    if hits.is_empty() {
        let _ = writeln!(out, "No results for \"{query}\"");
        return Ok(());
    }
    for hit in hits {
        let _ = writeln!(out, "{}\t{}", hit.id, hit.title);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use crate::model::{Book, SearchHit};

    struct FakeSearch {
        hits: Vec<SearchHit>,
    }

    impl BookSearch for FakeSearch {
        fn search(&self, _query: &str) -> Result<Vec<SearchHit>> {
            Ok(self.hits.clone())
        }

        fn fetch(&self, _id: &str) -> Result<Book> {
            unimplemented!("fetch is exercised in branch 2")
        }
    }

    #[test]
    fn test_search_command_prints_id_and_title() {
        let search = FakeSearch {
            hits: vec![SearchHit {
                id: "s5VfEAAAQBAJ".to_string(),
                title: "The Pragmatic Programmer".to_string(),
            }],
        };

        let mut out: Vec<u8> = Vec::new();
        run_search(&search, "pragmatic", &mut out).unwrap();

        let printed = String::from_utf8(out).unwrap();
        assert!(printed.contains("s5VfEAAAQBAJ"));
        assert!(printed.contains("The Pragmatic Programmer"));
    }
}
