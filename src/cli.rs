use std::io::Write;

use chrono::NaiveDate;
use clap::{Parser, Subcommand, ValueEnum};

use crate::error::Result;
use crate::model::Status;
use crate::repository::BookRepository;
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
    /// Add a book to your reading list by its Google Books id
    Add {
        id: String,
        #[arg(long, default_value = "to-read")]
        status: StatusArg,
        #[arg(long)]
        started: Option<NaiveDate>,
    },
}

// CLI-facing copy of Status so clap parsing stays out of the domain model. clap
// renders these kebab-case: read | reading | to-read.
#[derive(Clone, Copy, ValueEnum)]
pub enum StatusArg {
    Read,
    Reading,
    ToRead,
}

impl From<StatusArg> for Status {
    fn from(value: StatusArg) -> Self {
        match value {
            StatusArg::Read => Status::Read,
            StatusArg::Reading => Status::Reading,
            StatusArg::ToRead => Status::ToRead,
        }
    }
}

// Handler takes the search seam + a writer so it's unit-testable with a
// FakeSearch and a `Vec<u8>` buffer — no network, no stdout.
pub fn run_search(search: &impl BookSearch, query: &str, out: &mut impl Write) -> Result<()> {
    let hits = search.search(query)?;
    if hits.is_empty() {
        writeln!(out, "No results for \"{query}\"")?;
        return Ok(());
    }
    for hit in hits {
        writeln!(out, "{}\t{}", hit.id, hit.title)?;
    }
    Ok(())
}

// Add handler: fetch metadata for `id`, apply the chosen status (defaulting
// `started` to `today` when the book is being or has been read), and persist it.
// `today` is a parameter so the date logic stays deterministic under test.
pub fn run_add(
    repo: &mut impl BookRepository,
    search: &impl BookSearch,
    id: &str,
    status: Status,
    started: Option<NaiveDate>,
    today: NaiveDate,
    out: &mut impl Write,
) -> Result<()> {
    let mut book = search.fetch(id)?;
    book.status = status;
    book.started = match status {
        Status::Reading => Some(started.unwrap_or(today)),
        Status::Read => started,
        Status::ToRead => None,
    };
    book.completed = match status {
        Status::Read => Some(today),
        _ => None,
    };
    repo.add(book)?;
    writeln!(out, "Added {id}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use crate::model::{Book, SearchHit, Status};
    use crate::repository::InMemoryRepository;

    struct FakeSearch {
        hits: Vec<SearchHit>,
    }

    impl BookSearch for FakeSearch {
        fn search(&self, _query: &str) -> Result<Vec<SearchHit>> {
            Ok(self.hits.clone())
        }

        fn fetch(&self, id: &str) -> Result<Book> {
            Ok(Book {
                id: id.to_string(),
                title: "Fetched Title".to_string(),
                authors: vec![],
                status: Status::ToRead,
                started: None,
                completed: None,
                pages: None,
            })
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

    #[test]
    fn test_add_sets_started_when_reading() {
        let mut repo = InMemoryRepository::new();
        let search = FakeSearch { hits: vec![] };
        let today = NaiveDate::from_ymd_opt(2026, 6, 6).unwrap();
        let mut out: Vec<u8> = Vec::new();

        run_add(
            &mut repo,
            &search,
            "abc",
            Status::Reading,
            None,
            today,
            &mut out,
        )
        .unwrap();

        let book = repo.get("abc").unwrap().unwrap();
        assert_eq!(book.status, Status::Reading);
        assert_eq!(book.started, Some(today));
    }

    #[test]
    fn test_add_read_sets_completed() {
        let mut repo = InMemoryRepository::new();
        let search = FakeSearch { hits: vec![] };
        let today = NaiveDate::from_ymd_opt(2026, 6, 6).unwrap();
        let mut out: Vec<u8> = Vec::new();

        run_add(
            &mut repo,
            &search,
            "abc",
            Status::Read,
            None,
            today,
            &mut out,
        )
        .unwrap();

        let book = repo.get("abc").unwrap().unwrap();
        assert_eq!(book.status, Status::Read);
        assert_eq!(book.completed, Some(today));
        assert_eq!(book.started, None);
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_search_propagates_write_errors() {
        let search = FakeSearch {
            hits: vec![SearchHit {
                id: "s5VfEAAAQBAJ".to_string(),
                title: "The Pragmatic Programmer".to_string(),
            }],
        };

        let err = run_search(&search, "pragmatic", &mut FailingWriter).unwrap_err();

        assert!(matches!(err, crate::error::BookError::Io(_)));
    }
}
