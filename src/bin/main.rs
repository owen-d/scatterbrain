//! Binary entrypoint for the scatterbrain tool

use scatterbrain::cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    cli::run().await
}
