use crate::error::Result;
use crate::model::{Book, SearchHit};
use serde::Deserialize;

pub trait BookSearch {
    fn search(&self, query: &str) -> Result<Vec<SearchHit>>;
    fn fetch(&self, id: &str) -> Result<Book>;
}

#[derive(Deserialize)]
struct SearchResponse {
    #[serde(default)]
    items: Vec<Volume>,
}

#[derive(Deserialize)]
struct Volume {
    id: String,
    #[serde(rename = "volumeInfo")]
    volume_info: VolumeInfo,
}

#[derive(Deserialize)]
struct VolumeInfo {
    title: String,
}

// Pure parsing seam: takes a Google Books `volumes` JSON body and pulls out the
// (id, title) pairs. Unit-tested without the network.
fn parse_search_response(json: &str) -> Result<Vec<SearchHit>> {
    let response: SearchResponse = serde_json::from_str(json)?;
    Ok(response
        .items
        .into_iter()
        .map(|volume| SearchHit {
            id: volume.id,
            title: volume.volume_info.title,
        })
        .collect())
}

pub struct GoogleBooks {
    api_key: Option<String>,
}

impl GoogleBooks {
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("GOOGLE_BOOKS_API_KEY").ok(),
        }
    }
}

impl Default for GoogleBooks {
    fn default() -> Self {
        Self::new()
    }
}

impl BookSearch for GoogleBooks {
    fn search(&self, query: &str) -> Result<Vec<SearchHit>> {
        let mut params = vec![("q", query)];
        if let Some(key) = &self.api_key {
            params.push(("key", key));
        }
        let response: String = reqwest::blocking::Client::new()
            .get("https://www.googleapis.com/books/v1/volumes")
            .query(&params)
            .send()?
            .error_for_status()?
            .text()?;
        parse_search_response(&response)
    }

    fn fetch(&self, id: &str) -> Result<Book> {
        let params: Vec<(&str, &str)> = self
            .api_key
            .as_deref()
            .map(|key| vec![("key", key)])
            .unwrap_or_default();
        let response: String = reqwest::blocking::Client::new()
            .get(format!("https://www.googleapis.com/books/v1/volumes/{id}"))
            .query(&params)
            .send()?
            .error_for_status()?
            .text()?;
        let volume: Volume = serde_json::from_str(&response)?;
        Ok(Book {
            id: volume.id,
            title: volume.volume_info.title,
            authors: vec![],                      // TODO: parse authors
            status: crate::model::Status::ToRead, // default to ToRead
            started: None,
            completed: None,
            pages: None,
        })
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
