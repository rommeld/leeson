use leeson::config::fetch_config;
use leeson::models::{Channel, ChannelLimits};
use leeson::websocket::{connect, ping, process_messages, subscribe, subscribe_instrument};

#[tokio::main]
async fn main() {
    let app_config = fetch_config().expect("Failed to load configuration.");

    let url = app_config.kraken.websocket_url;
    let symbol = vec!["BTC/USD".to_string()];

    let (mut write, mut read) = connect(url).await.unwrap();
    ping(&mut write).await.unwrap();
    subscribe(&mut write, &Channel::Ticker, &symbol)
        .await
        .unwrap();
    subscribe(&mut write, &Channel::Book, &symbol)
        .await
        .unwrap();
    subscribe(&mut write, &Channel::Candles, &symbol)
        .await
        .unwrap();
    subscribe(&mut write, &Channel::Trades, &symbol)
        .await
        .unwrap();
    subscribe_instrument(&mut write).await.unwrap();

    let limits = ChannelLimits {
        ticker: 5,
        book: 10,
        candle: 5,
        trade: 5,
        instrument: 5,
    };
    process_messages(&mut write, &mut read, &symbol, &limits)
        .await
        .unwrap();
}
