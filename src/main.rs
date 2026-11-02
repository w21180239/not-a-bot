use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = translatex::cli::Cli::parse();
    std::process::exit(translatex::cli::run(cli).await);
}
