use std::path::PathBuf;

use crate::error::Result;
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
        // TODO: return a clone of every stored book.
        todo!("Ok(self.books.clone())")
    }

    fn get(&self, id: &str) -> Result<Option<Book>> {
        // TODO: find the book whose id matches and return a clone.
        todo!("self.books.iter().find(|b| b.id == id).cloned(), wrapped in Ok")
    }

    fn add(&mut self, book: Book) -> Result<()> {
        // TODO:
        // 1. if any stored book already has book.id -> Err(BookError::DuplicateId(book.id)).
        // 2. otherwise push it and return Ok(()).
        todo!("dedupe on id, then push")
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

impl JsonRepository {
    // Opens the default store: BOOKCLI_STORE if set (tests point this at a temp
    // file), else dirs::config_dir()/bookcli/books.json.
    pub fn open() -> Result<Self> {
        // TODO: resolve the path -> env var BOOKCLI_STORE if present, else
        // dirs::config_dir() joined with "bookcli/books.json" -> then delegate to
        // Self::open_at(path).
        todo!("resolve store path, then call open_at")
    }

    // Opens (or starts) a store at an explicit path. Tests call this with a temp
    // file so they stay deterministic and isolated.
    pub fn open_at(path: PathBuf) -> Result<Self> {
        // TODO:
        // - if the file exists: read it and serde_json::from_str into your books
        //   (define a small `{ "books": [...] }` wrapper struct here that derives
        //   Serialize + Deserialize).
        // - if it doesn't exist: start with an empty Vec.
        // - return Self { path, books }.
        todo!("load existing books, or start empty")
    }

    // Persists the current books to `path` as pretty JSON, creating the parent
    // directory the first time.
    fn save(&self) -> Result<()> {
        // TODO: std::fs::create_dir_all on the parent, serialize the wrapper with
        // serde_json::to_string_pretty, write it to self.path.
        todo!("create parent dir, write pretty JSON")
    }
}

impl BookRepository for JsonRepository {
    fn all(&self) -> Result<Vec<Book>> {
        // TODO: clone self.books (same as the in-memory version).
        todo!("Ok(self.books.clone())")
    }

    fn get(&self, id: &str) -> Result<Option<Book>> {
        // TODO: find by id, clone.
        todo!("find by id")
    }

    fn add(&mut self, book: Book) -> Result<()> {
        // TODO:
        // 1. dedupe on id like InMemoryRepository.
        // 2. push, then call self.save()? so the change hits disk.
        todo!("dedupe, push, save")
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

        assert!(matches!(err, crate::error::BookError::DuplicateId(_)));
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
