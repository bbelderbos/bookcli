use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{BookError, Result};
use crate::model::Book;

// Storage seam. It grows across branches (update/delete land in branch 4, goals
// in branch 5); branch 2 only needs to add a book and read books back.
pub trait BookRepository {
    fn all(&self) -> Result<Vec<Book>>;
    fn get(&self, id: &str) -> Result<Option<Book>>;
    // Adds a book; errors with BookError::DuplicateId when the id already exists.
    fn add(&mut self, book: Book) -> Result<()>;
}

// In-memory test double: a plain Vec, no disk. Lets the add handler and the
// dedupe rule be unit-tested without touching the filesystem.
#[derive(Default)]
pub struct InMemoryRepository {
    books: Vec<Book>,
}

impl InMemoryRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BookRepository for InMemoryRepository {
    fn all(&self) -> Result<Vec<Book>> {
        Ok(self.books.clone())
    }

    fn get(&self, id: &str) -> Result<Option<Book>> {
        Ok(self.books.iter().find(|b| b.id == id).cloned())
    }

    fn add(&mut self, book: Book) -> Result<()> {
        if self.books.iter().any(|b| b.id == book.id) {
            return Err(BookError::DuplicateId(book.id));
        }
        self.books.push(book);
        Ok(())
    }
}

// File-backed store used by the real CLI. The on-disk shape is a small JSON
// wrapper (so it can grow when goals arrive in branch 5); for now it just holds
// the books. Mutations are written straight back to disk so they survive the
// one-shot process.
pub struct JsonRepository {
    path: PathBuf,
    books: Vec<Book>,
}

// On-disk shape: a `{ "books": [...] }` object rather than a bare array, so the
// file can gain sibling keys (goals in branch 5) without a migration. `#[serde(default)]`
// lets an absent `books` key deserialize to an empty Vec.
#[derive(Serialize, Deserialize, Default)]
struct StoreData {
    #[serde(default)]
    books: Vec<Book>,
}

impl JsonRepository {
    // Opens the default store: BOOKCLI_STORE if set (tests point this at a temp
    // file), else dirs::config_dir()/bookcli/books.json.
    pub fn open() -> Result<Self> {
        let path = match std::env::var("BOOKCLI_STORE") {
            Ok(p) => PathBuf::from(p),
            Err(_) => dirs::config_dir()
                .expect("no config dir")
                .join("bookcli")
                .join("books.json"),
        };
        Self::open_at(path)
    }

    // Opens (or starts) a store at an explicit path. Tests call this with a temp
    // file so they stay deterministic and isolated.
    pub fn open_at(path: PathBuf) -> Result<Self> {
        let books = if path.exists() {
            let text = std::fs::read_to_string(&path)?;
            let data: StoreData = serde_json::from_str(&text)?;
            data.books
        } else {
            Vec::new()
        };
        Ok(Self { path, books })
    }

    // Persists the current books to `path` as pretty JSON, creating the parent
    // directory the first time.
    fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = StoreData {
            books: self.books.clone(),
        };
        let json = serde_json::to_string_pretty(&data)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }
}

impl BookRepository for JsonRepository {
    fn all(&self) -> Result<Vec<Book>> {
        Ok(self.books.clone())
    }

    fn get(&self, id: &str) -> Result<Option<Book>> {
        Ok(self.books.iter().find(|b| b.id == id).cloned())
    }

    fn add(&mut self, book: Book) -> Result<()> {
        if self.books.iter().any(|b| b.id == book.id) {
            return Err(BookError::DuplicateId(book.id));
        }
        self.books.push(book);
        self.save()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Status;

    fn sample(id: &str) -> Book {
        Book {
            id: id.to_string(),
            title: "Some Title".to_string(),
            authors: vec![],
            status: Status::ToRead,
            started: None,
            completed: None,
            pages: None,
        }
    }

    #[test]
    fn test_add_book_persists() {
        let mut repo = InMemoryRepository::new();

        repo.add(sample("abc")).unwrap();

        assert_eq!(repo.all().unwrap().len(), 1);
        assert_eq!(repo.get("abc").unwrap().unwrap().id, "abc");
    }

    #[test]
    fn test_add_duplicate_id_errors() {
        let mut repo = InMemoryRepository::new();
        repo.add(sample("abc")).unwrap();

        let err = repo.add(sample("abc")).unwrap_err();

        assert!(matches!(err, BookError::DuplicateId(_)));
    }

    #[test]
    fn test_json_repo_roundtrip() {
        let path =
            std::env::temp_dir().join(format!("bookcli-roundtrip-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);

        {
            let mut repo = JsonRepository::open_at(path.clone()).unwrap();
            repo.add(sample("abc")).unwrap();
        }

        let repo = JsonRepository::open_at(path.clone()).unwrap();
        let books = repo.all().unwrap();

        assert_eq!(books.len(), 1);
        assert_eq!(books[0].id, "abc");

        let _ = std::fs::remove_file(&path);
    }
}
