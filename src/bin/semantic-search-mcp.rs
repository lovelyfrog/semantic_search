use clap::Parser;
use vnext_semantic_search::tools::mcp::{McpServerCli, run_mcp_server};

#[tokio::main]
async fn main() {
    let cli = McpServerCli::parse();
    if let Err(error) = run_mcp_server(cli).await {
        eprintln!("semantic-search-mcp error: {error}");
        std::process::exit(1);
    }
}
