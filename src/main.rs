use clap::Parser;
use vmrogue::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    vmrogue::run(cli).await
}
