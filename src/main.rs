use leeson::LeesonError;
use leeson::config::fetch_config;
use leeson::models::{Channel, ChannelLimits};
use leeson::websocket::{connect, ping, process_messages, subscribe, subscribe_instrument};

#[tokio::main]
async fn main() -> Result<(), LeesonError> {
    let app_config = fetch_config()?;

    let url = app_config.kraken.websocket_url;
    let symbol = vec!["BTC/USD".to_string()];

    let (mut write, mut read) = connect(url).await?;
    ping(&mut write).await?;
    subscribe(&mut write, &Channel::Ticker, &symbol).await?;
    subscribe(&mut write, &Channel::Book, &symbol).await?;
    subscribe(&mut write, &Channel::Candles, &symbol).await?;
    subscribe(&mut write, &Channel::Trades, &symbol).await?;
    subscribe_instrument(&mut write).await?;

    let limits = ChannelLimits {
        ticker: 5,
        book: 10,
        candle: 5,
        trade: 5,
        instrument: 5,
    };
    process_messages(&mut write, &mut read, &symbol, &limits).await?;

    Ok(())
}
