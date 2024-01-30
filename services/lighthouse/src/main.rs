use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // initialise tracing
    tracing_subscriber::fmt::init();
    Ok(())
}
