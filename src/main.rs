use anyhow::Result;
use llm_bucket::cli::{Cli, run};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment
    dotenv::dotenv().ok();

    // Initialize tracing for the CLI.
    tracing_subscriber::fmt::init();
    tracing::info!("CLI application startup: tracing initialised, environment loaded");

    let cli = Cli::parse();
    tracing::info!("CLI arguments parsed, invoking run");
    let result = run(cli).await;
    match &result {
        Ok(_) => tracing::info!("CLI completed successfully"),
        Err(e) => tracing::error!(error = %e, "CLI exited with error"),
    }
    result
}
