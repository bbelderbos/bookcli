use anyhow::Result;
use clap::Parser;

use bookcli::cli::{run_search, Cli, Command};
use bookcli::search::GoogleBooks;

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Command::Search { query } => {
            let search = GoogleBooks::new();
            let mut stdout = std::io::stdout().lock();
            run_search(&search, &query, &mut stdout)?;
        }
    }

    Ok(())
}
