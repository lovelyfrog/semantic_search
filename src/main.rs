use clap::Parser;
use vnext_semantic_search::tools::{Cli, run_cli};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match run_cli(cli).await {
        Ok(output) => {
            println!("{output}");
        }
        Err(error) => {
            eprintln!("semantic-search error: {error}");
            std::process::exit(1);
        }
    }
}
