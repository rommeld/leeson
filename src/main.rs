use leeson::LeesonError;
use leeson::auth::get_websocket_token;
use leeson::config::fetch_config;
use leeson::models::Channel;
use leeson::websocket::{
    connect, ping, process_messages, subscribe, subscribe_executions, subscribe_instrument,
};

#[tokio::main]
async fn main() -> Result<(), LeesonError> {
    // Initialize tracing subscriber for logging output.
    tracing_subscriber::fmt::init();

    let app_config = fetch_config()?;

    let url = &app_config.kraken.websocket_url;
    let symbols = vec!["BTC/USD".to_string()];

    // Fetch a WebSocket token if API credentials are configured.
    let token = match (&app_config.kraken.api_key, &app_config.kraken.api_secret) {
        (Some(key), Some(secret)) if !key.is_empty() && !secret.is_empty() => {
            Some(get_websocket_token(key, secret).await?)
        }
        _ => None,
    };

    let (mut write, mut read) = connect(url).await?;
    ping(&mut write).await?;
    subscribe(&mut write, &Channel::Ticker, &symbols, None).await?;
    subscribe(&mut write, &Channel::Book, &symbols, None).await?;
    subscribe(&mut write, &Channel::Candles, &symbols, None).await?;
    subscribe(&mut write, &Channel::Trades, &symbols, None).await?;
    subscribe_instrument(&mut write).await?;

    if let Some(ref token) = token {
        subscribe(&mut write, &Channel::Orders, &symbols, Some(token)).await?;
        subscribe_executions(&mut write, token, true, false).await?;
    }

    process_messages(&mut read).await?;

    Ok(())
}
