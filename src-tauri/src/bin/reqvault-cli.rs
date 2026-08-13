#[tokio::main]
async fn main() -> std::process::ExitCode {
    reqvault_lib::cli::run(std::env::args()).await
}
