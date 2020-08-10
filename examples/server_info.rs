use anyhow::*;
use repulse::Client;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect().await
        .context("Failed to create client")?;

    let server_info = client.get_server_info().await?;

    println!("{:#?}", server_info);

    Ok(())
}
