#[tokio::main]
async fn main() {
    std::process::exit(not_a_bot::cli::main().await);
}
