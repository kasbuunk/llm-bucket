use anyhow::Result;
use llm_bucket::{Cli, run};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    // Initialize tracing for the CLI.
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    run(cli).await
}
