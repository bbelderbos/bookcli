use anyhow::Result;
use clap::Parser;

use bookcli::cli::{run_add, run_search, Cli, Command};
use bookcli::repository::JsonRepository;
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
        Command::Add {
            id,
            status,
            started,
        } => {
            let search = GoogleBooks::new();
            let mut repo = JsonRepository::open()?;
            let today = chrono::Local::now().date_naive();
            let mut stdout = std::io::stdout().lock();
            run_add(
                &mut repo,
                &search,
                &id,
                status.into(),
                started,
                today,
                &mut stdout,
            )?;
        }
    }

    Ok(())
}
