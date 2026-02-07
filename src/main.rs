use leeson::LeesonError;
use leeson::config::fetch_config;
use leeson::models::Channel;
use leeson::websocket::{connect, ping, process_messages, subscribe, subscribe_instrument};

#[tokio::main]
async fn main() -> Result<(), LeesonError> {
    // Initialize tracing subscriber for logging output.
    tracing_subscriber::fmt::init();

    let app_config = fetch_config()?;

    let url = &app_config.kraken.websocket_url;
    let symbols = vec!["BTC/USD".to_string()];

    let (mut write, mut read) = connect(url).await?;
    ping(&mut write).await?;
    subscribe(&mut write, &Channel::Ticker, &symbols).await?;
    subscribe(&mut write, &Channel::Book, &symbols).await?;
    subscribe(&mut write, &Channel::Candles, &symbols).await?;
    subscribe(&mut write, &Channel::Trades, &symbols).await?;
    subscribe_instrument(&mut write).await?;

    process_messages(&mut read).await?;

    Ok(())
}
