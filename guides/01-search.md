# Branch 1 — `search` (foundations)

The first branch sets the shape every later branch reuses: typed errors, a trait
that hides I/O behind a seam, pure parsing functions, and handlers that take a
writer so they test without touching the terminal. This guide explains *why* each
piece looks the way it does, and walks each stub so you can fill it in.

```
book search "the pragmatic programmer"
```

The scaffold is already in place: the types and signatures exist, the tests are
written and failing, and the function bodies are `todo!()`. Your job is to turn the
stubs into green.

Run them red first to see where you stand:

```bash
cargo test parse        # parser tests, red
cargo test search       # empty-results + command tests, red
```

---

## The big idea: push I/O to the edges

A CLI is mostly glue between the outside world (network, disk, stdout) and a bit of
logic. If the logic and the I/O are tangled, you can only test by hitting Google and
capturing real stdout — slow, flaky, offline-hostile.

So we split every feature into three layers:

1. **A trait** that names the I/O operation (`BookSearch`) — the *seam*.
2. **A pure function** that does the thinking (`parse_search_response`) — no I/O.
3. **A handler** that orchestrates, writing to an injected `Write` sink (`run_search`).

The real program wires the slow implementation (`GoogleBooks`) into the handler.
Tests wire a fake (`FakeSearch`) and a `Vec<u8>` buffer. Same handler, no network.

Keep this mental model — branches 2–6 are the same pattern with different nouns.

---

## Errors as data (`src/error.rs`)

This part is scaffold — the enum is given. Read it, you'll lean on it in every body:

```rust
#[derive(Debug, Error)]
pub enum BookError {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("failed to write output: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, BookError>;
```

Three things worth internalizing:

- **One error enum per library.** Each variant is a *category* of failure, not a
  message string. Callers can `match` on `BookError::Http(_)` if they ever need to;
  a `String` error would force them to parse text.
- **`#[from]` generates `From` impls.** That's what makes the `?` operator work. When
  `serde_json::from_str` returns `serde_json::Error`, `?` calls
  `From<serde_json::Error> for BookError` (auto-derived by the `Parse` variant) and
  returns early. You never write `.map_err(...)` for these.
- **`#[error("...")]`** is the `Display` text — what the user sees. `thiserror`
  writes the boilerplate `impl Display` and `impl std::error::Error` for you.

The `Result<T>` alias means every signature reads `Result<Vec<SearchHit>>` instead of
`std::result::Result<Vec<SearchHit>, BookError>`. Small, but it adds up.

**Where library vs application errors meet:** `main.rs` returns
`anyhow::Result<()>`, not `BookError`. `anyhow` swallows any `std::error::Error`, so
`?` on a `BookError` "just works" at the top. The rule of thumb: **typed errors
(`thiserror`) inside the library, `anyhow` at the binary boundary.**

---

## The domain model (`src/model.rs`)

Also scaffold — the types are defined for you:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status { Read, Reading, ToRead }
```

`Status` is `Copy` because it's three bytes of nothing — copying is cheaper than
managing references. `#[serde(rename_all = "snake_case")]` is the bridge between
Rust's `ToRead` and JSON's `"to_read"`; you write idiomatic Rust, the wire stays
conventional.

```rust
pub struct SearchHit { pub id: String, pub title: String }
```

Search returns the *minimal* pair — an id and a title. Full metadata (authors,
pages) is only fetched on `add`, where you actually commit a book to the list.
Modeling the lightweight result separately keeps `search` cheap and honest about
what a search response actually contains.

Note `SearchHit` has no `serde` derives — it never touches JSON directly. We
deserialize into private wire structs (below) and *map* into `SearchHit`. Don't
derive traits a type doesn't need.

---

## The search seam (`src/search.rs`)

The trait is given — it names the operations and defers the *how*:

```rust
pub trait BookSearch {
    fn search(&self, query: &str) -> Result<Vec<SearchHit>>;
    fn fetch(&self, id: &str) -> Result<Book>;
}
```

Two methods: `search` (branch 1) and `fetch` (used by `add` in branch 2). One trait,
because they're the same external dependency (Google Books). Handlers will take
`&impl BookSearch`, so they accept *any* implementation — real or fake.

### Wire structs are private

These mirror Google's JSON shape, and nothing more. They're **private to the module**
because they're an implementation detail of "how Google happens to format things" —
not part of our domain. The struct definitions are given; the two `serde` attributes
are the whole reason they exist:

```rust
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
```

- `#[serde(default)]` on `items`: if the JSON has no `items` key (a zero-result
  search), use `Vec::default()` (empty) instead of erroring. This is why
  `test_search_empty_results` passes without special-casing.
- `#[serde(rename = "volumeInfo")]`: Google uses camelCase; we map it to a snake_case
  Rust field at the boundary.

### Pure parsing — the testable core

```rust
fn parse_search_response(json: &str) -> Result<Vec<SearchHit>> {
    // 1. serde_json::from_str the json into a SearchResponse (? on failure)
    // 2. map each Volume into a SearchHit (id, volume_info.title)
    // 3. collect into Vec<SearchHit>
    todo!("deserialize into the wire struct, then map items -> SearchHit")
}
```

This function is the whole reason the design is testable. It takes a `&str` (not a
URL, not a socket) and returns hits. No network. `test_parse_google_search_response`
feeds it a saved fixture and asserts on the result — deterministic, no network.

Two idioms to reach for. The `?` after `from_str` leans on the `From` impl from the
error enum (`serde_json::Error -> BookError::Parse`) — no `.map_err`. And in the
mapping, prefer `.into_iter()` over `.iter()`: it *consumes* `response`, so you
*move* each `id` and `title` `String` out instead of cloning. Since you own
`response` and never use it again, moving is free. Reach for `into_iter` when you're
done with the source. That's `test_parse_google_search_response` and
`test_search_empty_results` green.

### The real implementation

`GoogleBooks` is the I/O edge — the only place that actually talks to the network:

```rust
impl BookSearch for GoogleBooks {
    fn search(&self, query: &str) -> Result<Vec<SearchHit>> {
        // build query params (q=query, plus key= if self.api_key is Some)
        // GET the volumes endpoint, .send()?.error_for_status()?.text()?
        // hand the body to parse_search_response
        todo!("fetch the text, then delegate to the pure parser")
    }
    // fetch is analogous — covered when branch 2 needs it
}
```

`GoogleBooks` holds an optional API key (read from `GOOGLE_BOOKS_API_KEY` in `new()`).
Search works without one; the key just lifts Google's anonymous rate limit, so push it
in only `if let Some(key) = &self.api_key`. Build a `Vec` of `("q", query)` params and
conditionally `push` the key.

The method should do I/O and *nothing else clever* — fetch the text and hand it to the
pure parser. That's deliberate: the only untestable line is the HTTP call; all the
logic lives in `parse_search_response`, where tests can reach it. The endpoint is
`https://www.googleapis.com/books/v1/volumes` and `reqwest::blocking::Client` exposes
`.get(url).query(&params).send()`.

Two `?`s worth naming as you assemble the chain:

- `.error_for_status()?` turns an HTTP 4xx/5xx into a `reqwest::Error`, which `?`
  converts to `BookError::Http`. Without it, a 429 rate-limit would be parsed as if it
  were a book list. Don't skip it.
- `reqwest::blocking` — use the *blocking* client on purpose. A one-shot CLI makes
  one request and exits; an async runtime would be ceremony for no gain (see the
  spec's non-goals).

---

## The handler (`src/cli.rs`)

```rust
pub fn run_search(search: &impl BookSearch, query: &str, out: &mut impl Write) -> Result<()> {
    // 1. search.search(query)? -> hits
    // 2. if empty: writeln! a "no results" line and return early
    // 3. otherwise: for each hit, writeln! "{id}\t{title}"
    todo!("query the seam, then write each hit to the injected sink")
}
```

The two parameters are what make this testable:

- `search: &impl BookSearch` — the seam. Production passes `GoogleBooks`, tests pass
  `FakeSearch`. The handler can't tell the difference.
- `out: &mut impl Write` — the output sink. Production passes a locked stdout, tests
  pass a `Vec<u8>` and then assert on the bytes. The handler never names stdout.

Three things the body needs. Guard the empty case first (`hits.is_empty()`) with an
early `return Ok(())` after writing a friendly line — `test_search_empty_results`
relies on that path. Every `writeln!(...)?` can fail (a closed pipe, e.g.
`book search x | head`), and that `?` turns it into `BookError::Io` — propagating
write errors instead of ignoring them is why `test_search_propagates_write_errors`
exists, so keep the `?` on every write. And make the output tab-separated
(`"{}\t{}"`, id then title): it's intentional, so it stays pipe-friendly
(`book search rust | cut -f1`). That greens `test_search_command_prints_id_and_title`
and `test_search_propagates_write_errors`.

### `main.rs` does the wiring

`main` is the only place that picks the *real* implementations, so it's shown whole —
it's glue, not a lesson:

```rust
Command::Search { query } => {
    let search = GoogleBooks::new();
    let mut stdout = std::io::stdout().lock();
    run_search(&search, &query, &mut stdout)?;
}
```

It locks stdout once (one lock, not one per line) and hands both to the handler.
Notice `main` has no logic — just construction and delegation. That's the goal.

---

## Testing without the world (`src/cli.rs` tests)

The tests are written for you — read them to see what "green" means. The doubles are
the lesson:

- **`FakeSearch`** implements `BookSearch` by returning canned `hits` and ignoring the
  query. That's enough to test the handler's *behavior* (does it print id and title?
  does it propagate write errors?) without a single packet leaving the machine. Its
  `fetch` is `unimplemented!()` here because branch 1 never calls it — it gets a real
  fake in branch 2.
- **`FailingWriter`** is a `Write` whose `write()` always returns `BrokenPipe`,
  proving the handler surfaces I/O errors rather than swallowing them.

Fixtures (`tests/fixtures/google_*.json`) are real-shaped Google responses saved to
disk and pulled in with `include_str!` — so the parser is tested against the actual
format, not a hand-waved guess.

---

## Verify, then update the README

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test            # all branch-1 tests green
```

Then add `book search` to `README.md` — usage and a one-line description. The README
should advertise a command only once it actually works (no `todo!()`).

Optional smoke test against the real API:

```bash
cargo run -- search "the pragmatic programmer"
```

---

## What to carry into the next branches

- **Trait = seam.** Every external dependency (now HTTP, next disk) hides behind a
  trait so handlers stay testable.
- **Pure function = the part worth testing.** Keep logic out of the I/O methods.
- **Inject the sink.** `&mut impl Write` beats hard-coded `println!` every time.
- **`thiserror` in the lib, `anyhow` in `main`.**
- **`?` + `#[from]`** removes almost all manual error plumbing.

Branch 2 (`add`) introduces a second seam — `BookRepository` for storage — and reuses
every habit above. See `guides/02-add.md`.
