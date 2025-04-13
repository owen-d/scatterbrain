//! Binary entrypoint for the scatterbrain tool

use scatterbrain::{
    api::{serve, ServerConfig},
    Context, Core, Plan, DEFAULT_LEVELS,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting scatterbrain API server...");

    // Create a default plan with the default levels
    let plan = Plan::new(DEFAULT_LEVELS.to_vec());
    let context = Context::new(plan);
    let core = Core::new(context);

    // Create a default server configuration
    let config = ServerConfig::default();
    println!("Server will listen on {}", config.address);

    // Start the API server
    serve(core, config).await
}
