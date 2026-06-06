# bookcli

Track your reading from the terminal. Searches Google Books and (soon) keeps a local reading list.

## Usage

```bash
cargo run -- search "the pragmatic programmer"
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
