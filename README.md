# bookcli

Track your reading from the terminal. Searches Google Books and keeps a local reading list.

## Usage

Search Google Books for a query — prints each match as `id<TAB>title`:

```bash
cargo run -- search "the pragmatic programmer"
```

Add a book to your reading list by its Google Books id. `--status` defaults to
`to-read`; the date stamped follows the status — `reading` sets `started`
(today, or `--started`), `read` sets `completed` to today:

```bash
cargo run -- add s5VfEAAAQBAJ --status reading
cargo run -- add s5VfEAAAQBAJ --status read --started 2026-01-10
```

## Google Books API key

Search works without a key, but Google rate-limits anonymous requests. For
reliable use, set a `GOOGLE_BOOKS_API_KEY`.

Get one:

1. Open the [Google Cloud Console](https://console.cloud.google.com/) and create
   (or select) a project.
2. Under **APIs & Services → Library**, enable the **Books API**.
3. Under **APIs & Services → Credentials**, click **Create credentials → API key**.

Then make it available to the CLI in one of two ways:

- **`.env` file** (loaded automatically via `dotenvy`) — create a `.env` in the
  project root:

  ```
  GOOGLE_BOOKS_API_KEY=your-key-here
  ```

- **Shell export** — for the current session:

  ```bash
  export GOOGLE_BOOKS_API_KEY=your-key-here
  ```

The key is optional; without it the CLI still runs, just unauthenticated.
