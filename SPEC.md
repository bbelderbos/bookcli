# Books CLI (`bookcli`) — Spec

A small Rust CLI to track your reading. Search Google Books, add books with a
reading status, list and get stats, set a yearly reading goal, and import an
existing PyBites Books reading list.

Built as a **6-part learning progression** (one git branch per part), **TDD-first**,
with a **repository trait** so the storage backend is swappable and testable.

---

## Goals

- Teach idiomatic Rust through a real, useful CLI.
- **Hybrid-learning:** each branch ships as stubs + failing tests; the student writes
  the bodies, with a guide that explains the *approach* but never the answer. Assembling
  the body from the idea is the point — go slow, keep the friction, don't delegate it.
- Each part is a self-contained branch that ends green (tests + `clippy` clean).
- Domain logic is pure and unit-tested behind traits; I/O lives at the edges.

## Non-goals

- No multi-user, no auth, no server. Single local user.
- No DB. JSON file storage only.
- No async. Blocking HTTP is enough for a one-shot CLI.

---

## Tech stack

| Concern        | Choice                  | Why |
|----------------|-------------------------|-----|
| CLI parsing    | `clap` (derive)         | Standard, ergonomic subcommands |
| HTTP           | `reqwest` (blocking)    | One request at a time; no runtime |
| Serialization  | `serde` + `serde_json`  | Model + JSON store |
| HTML scraping  | `scraper`               | Branch 6 parses the public user page |
| Dates          | `chrono`                | `started` / `completed` timestamps, year math |
| Config path    | `dirs`                  | XDG config dir for the store |
| Errors         | `thiserror` + `anyhow`  | Typed lib errors; `anyhow` at the `main` boundary |

Crate name `bookcli`, binary name `book` (so commands read `book search ...`).

---

## Storage

Single JSON file at the platform config dir:

- Linux: `~/.config/bookcli/books.json`
- macOS: `~/Library/Application Support/bookcli/books.json`

Resolved via `dirs::config_dir()`. Created on first write. Overridable with
`BOOKCLI_STORE` env var (used by tests to point at a temp file).

```json
{
  "books": [
    {
      "id": "Yt5bDAAAQBAJ",
      "title": "Nobody Wants to Read Your Sh*t",
      "authors": ["Steven Pressfield"],
      "status": "read",
      "started": "2024-01-10",
      "completed": "2024-01-20",
      "pages": 211
    }
  ],
  "goals": [
    { "year": 2026, "target": 50 }
  ]
}
```

---

## Domain model

```rust
enum Status { Read, Reading, ToRead }      // serde: "read" | "reading" | "to_read"

struct Book {
    id: String,            // Google Books volume id (primary key, unique)
    title: String,
    authors: Vec<String>,
    status: Status,
    started: Option<NaiveDate>,    // set when status is Reading; on Read only if given
    completed: Option<NaiveDate>,  // set to today when status is Read
    pages: Option<u32>,
}

struct Goal { year: i32, target: u32 }
```

A lightweight `SearchHit { id, title }` is returned by search (branch 1) — the
minimal pair from the mindmap. Full metadata is fetched on `add`.

---

## Architecture — traits (the repo pattern)

Two seams keep domain logic pure and unit-tested without I/O (no network, no filesystem):

```rust
// Storage seam — branches 2-6
trait BookRepository {
    fn all(&self) -> Result<Vec<Book>>;
    fn get(&self, id: &str) -> Result<Option<Book>>;
    fn add(&mut self, book: Book) -> Result<()>;      // errors if id exists
    fn update(&mut self, id: &str, status: Status) -> Result<()>;
    fn delete(&mut self, id: &str) -> Result<()>;

    fn goals(&self) -> Result<Vec<Goal>>;
    fn set_goal(&mut self, goal: Goal) -> Result<()>; // upsert by year
    fn delete_goal(&mut self, year: i32) -> Result<()>;
}

// HTTP seam — branch 1 (and 2's metadata fetch)
trait BookSearch {
    fn search(&self, query: &str) -> Result<Vec<SearchHit>>;
    fn fetch(&self, id: &str) -> Result<Book>;
}
```

Implementations:
- `JsonRepository` — file-backed, used by the real CLI.
- `InMemoryRepository` — `Vec<Book>` test double for unit tests.
- `GoogleBooks` — real `reqwest` client.
- `FakeSearch` — canned responses in tests.

Command handlers take `&mut impl BookRepository` / `&impl BookSearch`, so every
handler is unit-testable without touching the network or disk.

---

## CLI surface

```
book search <query>                      # 1: Google Books -> id + title list
book add <id> [--status read|reading|to-read] [--started YYYY-MM-DD]   # 2
book list [--status read|reading|to-read]                              # 3
book update <id> --status <read|reading|to-read>                       # 4
book delete <id>                                                       # 4
book goal set <target> [--year YYYY]     # 5  (defaults to current year)
book goal show [--year YYYY]             # 5
book goal delete [--year YYYY]           # 5
book import <username>                   # 6: scrape pybitesbooks.com/users/<username>
```

`--status` default on `add` is `to-read`. Date defaults follow the status —
adding a `read` book stamps `completed`, not `started`:

| status    | `started`                  | `completed` |
|-----------|----------------------------|-------------|
| `reading` | today (or `--started`)     | `None`      |
| `read`    | `--started` if given, else `None` | today |
| `to-read` | `None`                     | `None`      |

---

## External data sources

### Google Books API (no key needed)
- Search: `GET https://www.googleapis.com/books/v1/volumes?q=<query>`
  → `items[].id`, `items[].volumeInfo.title`
- Fetch one: `GET https://www.googleapis.com/books/v1/volumes/<id>`
  → `volumeInfo.{title, authors, pageCount}`

### PyBites Books import (branch 6 — scrape)
Public page `https://pybitesbooks.com/users/<username>` exposes everything we
need, so **no Django changes required**. Structure:
- `<div id="reading-books">` → status `reading`
- `<div id="completed-books">` → status `read`
- Want-to-Read section → status `to-read`
- Each card: `<a href="/books/<id>" title="<title>">` (id + title)

Scrape with `scraper` CSS selectors; map section → status; upsert into the store.
(Fallback if scraping ever breaks: add a `/api/userbooks/<username>` JSON endpoint
to the Django app and consume that instead.)

---

## The 6 branches

Each branch: **write the failing tests first**, implement to green, `cargo fmt`,
`cargo clippy -- -D warnings`, `cargo test`, then merge.

### Branch 1 — `search` (foundations)
Project setup, error types, `BookSearch` trait, `GoogleBooks` client, `search` cmd.
- `test_parse_google_search_response` — fixture JSON → `Vec<SearchHit>`
- `test_search_empty_results` → empty vec, no panic
- `test_search_command_prints_id_and_title` (via `FakeSearch`)

### Branch 2 — `add` (persistence + repo trait)
`Status`, `Book`, `BookRepository`, `JsonRepository`, `InMemoryRepository`, dedupe.
- `test_add_book_persists`
- `test_add_duplicate_id_errors`
- `test_add_sets_started_when_reading`
- `test_add_read_sets_completed`
- `test_json_repo_roundtrip` (write then read back from temp file)

### Branch 3 — `list` + stats
Read from repo, optional status filter, aggregate counts and pages.
- `test_list_all` / `test_list_filtered_by_status`
- `test_stats_counts_by_status`
- `test_stats_total_pages_read`

### Branch 4 — `update` + `delete`
Mutate status by id; remove by id; clear errors on missing id.
- `test_update_status`
- `test_update_to_read_sets_completed`
- `test_delete_book`
- `test_update_missing_id_errors` / `test_delete_missing_id_errors`

### Branch 5 — `goal`
Yearly target: set (upsert), show (progress = books completed that year / target), delete.
- `test_set_and_get_goal`
- `test_goal_progress_counts_completed_this_year`
- `test_set_goal_overwrites_same_year`
- `test_delete_goal`

### Branch 6 — `import`
Scrape a public PyBites user page → books into the store, skipping ones already present.
- `test_parse_user_page_extracts_books` (fixture HTML → `Vec<Book>` with statuses)
- `test_import_skips_existing`
- `test_import_maps_sections_to_status`

---

## Verification loop (every branch)

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

---

## Open questions / later

- Categories, ISBN, publisher: in the model? (kept out for now — minimal.)
- `search` interactive pick → `add` in one step? (out of scope; two commands.)
- Import: scrape first; promote to a JSON API endpoint only if the page changes.
