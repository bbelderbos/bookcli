use crate::error::Result;
use crate::model::{Book, SearchHit};

pub trait BookSearch {
    fn search(&self, query: &str) -> Result<Vec<SearchHit>>;
    fn fetch(&self, id: &str) -> Result<Book>;
}

// Pure parsing seam: takes a Google Books `volumes` JSON body and pulls out the
// (id, title) pairs. Unit-tested without the network.
fn parse_search_response(json: &str) -> Result<Vec<SearchHit>> {
    todo!("deserialize the body, map items[].id + items[].volumeInfo.title into SearchHit; missing `items` => empty vec")
}

pub struct GoogleBooks;

impl GoogleBooks {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoogleBooks {
    fn default() -> Self {
        Self::new()
    }
}

impl BookSearch for GoogleBooks {
    fn search(&self, query: &str) -> Result<Vec<SearchHit>> {
        todo!("GET googleapis.com/books/v1/volumes?q={query}, then parse_search_response")
    }

    fn fetch(&self, id: &str) -> Result<Book> {
        todo!("GET googleapis.com/books/v1/volumes/{id}, map volumeInfo into a Book")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH_FIXTURE: &str = include_str!("../tests/fixtures/google_search.json");

    #[test]
    fn test_parse_google_search_response() {
        let hits = parse_search_response(SEARCH_FIXTURE).unwrap();

        assert_eq!(hits.len(), 2);
        assert_eq!(
            hits[0],
            SearchHit {
                id: "s5VfEAAAQBAJ".to_string(),
                title: "The Pragmatic Programmer".to_string(),
            }
        );
        assert_eq!(hits[1].id, "hjEFCAAAQBAJ");
        assert_eq!(hits[1].title, "Clean Code");
    }

    #[test]
    fn test_search_empty_results() {
        let json = r#"{ "kind": "books#volumes", "totalItems": 0 }"#;

        let hits = parse_search_response(json).unwrap();

        assert!(hits.is_empty());
    }
}
