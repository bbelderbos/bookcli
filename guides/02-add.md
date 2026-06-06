# Branch 2 — `add` (persistence + the repository pattern)

Branch 1 read from the network. Branch 2 *writes* to disk. The new idea is a second
seam — `BookRepository` — that hides storage the way `BookSearch` hid HTTP. By the
end, `book add <id>` fetches metadata, stamps a status and date, and saves a book to
a JSON file, refusing duplicates.

```
book add s5VfEAAAQBAJ --status reading
book add hjEFCAAAQBAJ --status read --started 2024-01-10
```

The scaffold is already in place: tests written and failing, function bodies left as
`todo!()`. This guide explains the concept behind each stub so you can fill it in
(with Copilot) and turn the 4 failing tests green.

Run them red first to see where you stand:

```bash
cargo test repository   # 3 red
cargo test cli::tests::test_add_sets_started_when_reading   # 1 red
```

---

## The storage seam (`src/repository.rs`)

```rust
pub trait BookRepository {
    fn all(&self) -> Result<Vec<Book>>;
    fn get(&self, id: &str) -> Result<Option<Book>>;
    fn add(&mut self, book: Book) -> Result<()>;
}
```

Same move as `BookSearch`: name the operations, defer the *how*. Two implementations:

- **`InMemoryRepository`** — a `Vec<Book>`, no disk. The fast test double.
- **`JsonRepository`** — file-backed, used by the real CLI.

Handlers will take `&mut impl BookRepository`, so they work against either. Unit tests
use the in-memory one; only the roundtrip test touches a real file.

Why does `add` take `&mut self` but `all`/`get` take `&self`? Adding *mutates* the
store; reading doesn't. Encoding that in the signature lets the compiler enforce it —
you literally cannot call `add` through a shared reference. The borrow checker is
documentation that can't go stale.

The trait will **grow** in later branches (`update`/`delete` in 4, goals in 5). We add
only the three methods this branch needs — adding stubs for unused methods now would
mean dead `todo!()`s and no tests to justify them. One branch at a time.

---

## `InMemoryRepository` — start here

It's the simplest implementation and unblocks two tests
(`test_add_book_persists`, `test_add_duplicate_id_errors`).

```rust
fn all(&self) -> Result<Vec<Book>> {
    todo!("Ok(self.books.clone())")
}
```

We return an owned `Vec<Book>` (a clone), not `&[Book]`. The trait promises a value so
that `JsonRepository`, which builds its list freshly, can satisfy the same signature.
Cloning a handful of books in a CLI is free; don't optimize it away.

```rust
fn get(&self, id: &str) -> Result<Option<Book>> {
    todo!("self.books.iter().find(|b| b.id == id).cloned(), wrapped in Ok")
}
```

`Option<Book>` is the honest return type: a lookup either finds a book or doesn't —
that's not an *error*, it's a normal outcome. `.find()` returns `Option<&Book>`;
`.cloned()` turns it into `Option<Book>`. Reserve `Err` for things that actually went
wrong (a duplicate, a failed write) — not for "no match".

```rust
fn add(&mut self, book: Book) -> Result<()> {
    // 1. if any stored book already has book.id -> Err(BookError::DuplicateId(book.id))
    // 2. otherwise push it
    todo!("dedupe on id, then push")
}
```

The dedupe is the lesson here. `id` is the primary key (Google's volume id), so two
copies of the same book is a bug, not a merge. Check first:

```rust
if self.books.iter().any(|b| b.id == book.id) {
    return Err(BookError::DuplicateId(book.id));
}
self.books.push(book);
Ok(())
```

`.any()` short-circuits on the first match. Note you move `book.id` into the error
*only* on the failure path; on success you `push(book)` whole. The borrow checker
keeps you honest — you can't use `book` after moving a field out of it, so the early
`return` is the natural shape.

That's `test_add_book_persists` and `test_add_duplicate_id_errors` green.

---

## `JsonRepository` — persistence

This is where the real work is. The on-disk shape is a small JSON wrapper so it can
grow when goals arrive in branch 5:

```json
{ "books": [ { "id": "...", "title": "...", "status": "to_read" } ] }
```

You'll define that wrapper *inside the impl* — it's a storage detail, private:

```rust
#[derive(Serialize, Deserialize, Default)]
struct StoreData {
    #[serde(default)]
    books: Vec<Book>,
}
```

`#[serde(default)]` on `books` means an empty or partial file still deserializes —
and when branch 5 adds a `goals` field, *old* files without it still load. Forward
compatibility for free. (Add `use serde::{Serialize, Deserialize};` at the top.)

### Two constructors, on purpose

```rust
pub fn open() -> Result<Self> {
    // BOOKCLI_STORE env var, else dirs::config_dir()/bookcli/books.json
    todo!("resolve store path, then call open_at")
}

pub fn open_at(path: PathBuf) -> Result<Self> {
    // load existing books or start empty
    todo!()
}
```

`open()` resolves *where* the file lives; `open_at(path)` does the actual loading. The
split exists for testability: `test_json_repo_roundtrip` calls `open_at` with a temp
path directly, so tests never depend on your machine's config dir — and never mutate a
process-global env var, which would race under `cargo test`'s parallel threads.

`open()`:

```rust
let path = match std::env::var("BOOKCLI_STORE") {
    Ok(p) => PathBuf::from(p),
    Err(_) => dirs::config_dir()
        .expect("no config dir")
        .join("bookcli")
        .join("books.json"),
};
Self::open_at(path)
```

`dirs::config_dir()` returns the right place per OS (`~/Library/Application Support`
on macOS, `~/.config` on Linux) — never hard-code `~/.config`. The `BOOKCLI_STORE`
override is what lets a real user (or a script) point at an alternate file.

`open_at()` — read if present, empty if not:

```rust
let books = if path.exists() {
    let text = std::fs::read_to_string(&path)?;
    let data: StoreData = serde_json::from_str(&text)?;
    data.books
} else {
    Vec::new()
};
Ok(Self { path, books })
```

First run has no file — that's not an error, it's an empty store. Both `?`s lean on
the `From` impls from branch 1: `std::io::Error -> BookError::Io`,
`serde_json::Error -> BookError::Parse`. You wrote zero error-mapping code.

### Saving

```rust
fn save(&self) -> Result<()> {
    todo!("create parent dir, write pretty JSON")
}
```

```rust
if let Some(parent) = self.path.parent() {
    std::fs::create_dir_all(parent)?;
}
let data = StoreData { books: self.books.clone() };
let json = serde_json::to_string_pretty(&data)?;
std::fs::write(&self.path, json)?;
Ok(())
```

`create_dir_all` is idempotent — it's fine if the dir already exists, and it makes the
*first* write succeed when `~/.../bookcli/` doesn't exist yet. `to_string_pretty`
keeps the file human-readable (you'll want to eyeball it while learning).

### `add` writes through

`JsonRepository::add` is `InMemoryRepository::add` plus a save:

```rust
fn add(&mut self, book: Book) -> Result<()> {
    if self.books.iter().any(|b| b.id == book.id) {
        return Err(BookError::DuplicateId(book.id));
    }
    self.books.push(book);
    self.save()
}
```

The dedupe check happens *before* the push, so a duplicate never gets written and
`save()` only runs on a real change. `all`/`get` are identical to the in-memory
versions (clone / find). That's `test_json_repo_roundtrip` green: one instance writes,
a fresh instance reads the same file back.

---

## The `add` handler (`src/cli.rs`)

```rust
pub fn run_add(
    repo: &mut impl BookRepository,
    search: &impl BookSearch,
    id: &str,
    status: Status,
    started: Option<NaiveDate>,
    today: NaiveDate,
    out: &mut impl Write,
) -> Result<()> {
    todo!("fetch -> set status/started -> repo.add -> confirm")
}
```

This handler touches *both* seams — it reads from `BookSearch` and writes to
`BookRepository` — yet it's still fully testable, because both are injected. The body:

```rust
let mut book = search.fetch(id)?;
book.status = status;
book.started = match status {
    Status::Reading | Status::Read => started.or(Some(today)),
    Status::ToRead => started,
};
repo.add(book)?;
writeln!(out, "Added {id}")?;
Ok(())
```

Three decisions worth understanding:

**Why fetch on add (not on search)?** Search returns lightweight `SearchHit`s. Only
when you commit a book do you spend a request to pull authors and page count. `fetch`
returns a `Book` already defaulted to `ToRead` (see branch 1's `parse_volume_response`),
and we override status/dates here.

**The `started` default.** A book you're *reading* or have *read* needs a start date;
a `to-read` book doesn't. `started.or(Some(today))` means "use what the user passed,
otherwise default to today" — but only for the two active statuses. `ToRead` keeps
whatever was passed (normally `None`).

**Why is `today` a parameter?** This is the testability lesson of the branch. If the
body called `chrono::Local::now()` directly, the test would compare against "now",
which changes every run — non-deterministic. Instead `main` injects the real today and
the test injects a *fixed* date:

```rust
let today = NaiveDate::from_ymd_opt(2026, 6, 6).unwrap();
run_add(&mut repo, &search, "abc", Status::Reading, None, today, &mut out)?;
let book = repo.get("abc").unwrap().unwrap();
assert_eq!(book.started, Some(today));   // deterministic
```

Pushing the clock to the edge (like we pushed the network and stdout to the edges) is
the same principle, applied to *time*. That's `test_add_sets_started_when_reading`
green — all four tests now pass.

---

## Keeping `clap` out of the domain (`StatusArg`)

```rust
#[derive(Clone, Copy, ValueEnum)]
pub enum StatusArg { Read, Reading, ToRead }

impl From<StatusArg> for Status {
    fn from(value: StatusArg) -> Self {
        match value {
            StatusArg::Read => Status::Read,
            StatusArg::Reading => Status::Reading,
            StatusArg::ToRead => Status::ToRead,
        }
    }
}
```

`model::Status` derives `serde` traits for *storage*. It deliberately does **not**
derive `clap::ValueEnum` — the domain model shouldn't know the CLI exists. So we keep
a CLI-local twin, `StatusArg`, that clap can parse, and convert at the boundary with
`From`. `clap` renders the variants kebab-case automatically: `read | reading |
to-read`, which is why `#[arg(long, default_value = "to-read")]` works.

`main` converts at the call site with `status.into()`. This is the same "translate at
the edge" habit as the private wire structs in branch 1 — the outside format
(`StatusArg`, Google JSON) never leaks into the core (`Status`, `Book`).

`--started YYYY-MM-DD` parses straight into `Option<NaiveDate>`: `chrono::NaiveDate`
implements `FromStr` for ISO dates, so clap needs no custom parser.

---

## Wiring it up (`src/main.rs`)

```rust
Command::Add { id, status, started } => {
    let search = GoogleBooks::new();
    let mut repo = JsonRepository::open()?;
    let today = chrono::Local::now().date_naive();
    let mut stdout = std::io::stdout().lock();
    run_add(&mut repo, &search, &id, status.into(), started, today, &mut stdout)?;
}
```

`main` is still pure wiring: pick the real implementations (`GoogleBooks`,
`JsonRepository::open()`), supply the real `today`, and delegate. The `?` after
`open()` converts a `BookError` into `anyhow::Error` at the binary boundary — the same
lib/app error split from branch 1.

---

## Verify, then update the README

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test            # all green: 6 from branch 1 + 4 from branch 2
```

Then add `book add` to `README.md` — usage and a one-line description. The README
should advertise a command only once it actually works (no `todo!()`).

Optional smoke test against the real file:

```bash
BOOKCLI_STORE=/tmp/books.json cargo run -- add s5VfEAAAQBAJ --status reading
cat /tmp/books.json
```

---

## What carried over, what's new

Carried from branch 1: trait-as-seam, inject the sink, `?` + `#[from]`, `thiserror`
in the lib / `anyhow` in `main`, translate foreign formats at the edge.

New in branch 2:
- A **second seam** (`BookRepository`) and two implementations behind it.
- **Inject the clock** (`today` param) — the time version of injecting I/O.
- **`Option` for "not found", `Err` for "went wrong"** — don't conflate them.
- **A forward-compatible on-disk format** via `#[serde(default)]`.

Branch 3 (`list` + stats) reads through the same repository — no new I/O, just
aggregation over what `add` stored.
