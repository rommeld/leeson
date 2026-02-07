use config::Config;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::Message;
use tungstenite::Result;

type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WsReader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

#[derive(Deserialize)]
struct AppConfig {
    kraken: KrakenConfig,
}

#[derive(Deserialize)]
struct KrakenConfig {
    websocket_url: String,
}

#[derive(Deserialize)]
struct TickerUpdateResponse {
    channel: String,
    #[serde(rename = "type")]
    tpe: String,
    data: Vec<TickerData>,
}

#[derive(Deserialize)]
struct TickerData {
    symbol: String,
    bid: f64,
    bid_qty: f64,
    ask: f64,
    ask_qty: f64,
    last: f64,
    volume: f64,
    vwap: f64,
    low: f64,
    high: f64,
    change: f64,
    change_pct: f64,
}

#[derive(Deserialize)]
struct BookUpdateResponse {
    channel: String,
    #[serde(rename = "type")]
    tpe: String,
    data: Vec<BookData>,
}

#[derive(Deserialize)]
struct BookData {
    symbol: String,
    bids: Vec<PriceLevel>,
    asks: Vec<PriceLevel>,
    checksum: u64,
    timestamp: String,
}

#[derive(Deserialize)]
struct PriceLevel {
    price: f64,
    qty: f64,
}

#[derive(Deserialize)]
struct OrdersUpdateResponse {
    channel: String,
    #[serde(rename = "type")]
    tpe: String,
    data: Vec<OrdersData>,
}

#[derive(Deserialize)]
struct OrdersData {
    symbol: String,
    bids: Vec<OrderEntry>,
    asks: Vec<OrderEntry>,
    checksum: u64,
    timestamp: String,
}

#[derive(Deserialize)]
struct OrderEntry {
    event: Option<String>,
    order_id: String,
    limit_price: f64,
    order_qty: f64,
    timestamp: String,
}

#[derive(Deserialize)]
struct CandleUpdateResponse {
    channel: String,
    #[serde(rename = "type")]
    tpe: String,
    timestamp: String,
    data: Vec<CandleData>,
}

#[derive(Deserialize)]
struct CandleData {
    symbol: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    vwap: f64,
    trades: u64,
    volume: f64,
    interval_begin: String,
    interval: u64,
    timestamp: String,
}

#[derive(Deserialize)]
struct TradeUpdateResponse {
    channel: String,
    #[serde(rename = "type")]
    tpe: String,
    data: Vec<TradeData>,
}

#[derive(Deserialize)]
struct TradeData {
    symbol: String,
    side: String,
    price: f64,
    qty: f64,
    ord_type: String,
    trade_id: u64,
    timestamp: String,
}

#[derive(Deserialize)]
struct InstrumentUpdateResponse {
    channel: String,
    #[serde(rename = "type")]
    tpe: String,
    data: InstrumentData,
}

#[derive(Deserialize)]
struct InstrumentData {
    assets: Vec<AssetInfo>,
    pairs: Vec<PairInfo>,
}

#[derive(Deserialize)]
struct AssetInfo {
    id: String,
    status: String,
    precision: u32,
    precision_display: u32,
    borrowable: bool,
    collateral_value: f64,
    margin_rate: f64,
}

#[derive(Deserialize)]
struct PairInfo {
    symbol: String,
    base: String,
    quote: String,
    status: String,
    qty_precision: u32,
    qty_increment: f64,
    price_precision: u32,
    price_increment: f64,
    cost_precision: u32,
    cost_min: String,
    qty_min: f64,
    marginable: bool,
    margin_initial: Option<f64>,
    position_limit_long: Option<u64>,
    position_limit_short: Option<u64>,
    has_index: bool,
}

#[derive(Deserialize)]
struct StatusUpdateResponse {
    channel: String,
    #[serde(rename = "type")]
    tpe: String,
    data: Vec<StatusData>,
}

#[derive(Deserialize)]
struct StatusData {
    api_version: String,
    connection_id: u64,
    system: String,
    version: String,
}

#[derive(Deserialize)]
struct HeartbeatResponse {
    channel: String,
}

#[derive(Serialize)]
struct PingRequest {
    method: String,
}

#[derive(Deserialize)]
struct PongResponse {
    method: String,
    time_in: String,
    time_out: String,
}

enum Channel {
    Book,
    Ticker,
    Orders,
    Candles,
    Trades,
    Instruments,
    Status,
    Heartbeat,
}

impl Channel {
    fn as_str(&self) -> &'static str {
        match self {
            Channel::Book => "book",
            Channel::Ticker => "ticker",
            Channel::Orders => "level3",
            Channel::Candles => "ohlc",
            Channel::Trades => "trade",
            Channel::Instruments => "instrument",
            Channel::Status => "status",
            Channel::Heartbeat => "heartbeat",
        }
    }
}

struct ChannelLimits {
    ticker: usize,
    book: usize,
    candle: usize,
    trade: usize,
    instrument: usize,
}

#[derive(Serialize)]
struct SubscribeRequest {
    method: String,
    params: Params,
}

#[derive(Serialize)]
struct UnsubscribeRequest {
    method: String,
    params: Params,
}

#[derive(Serialize)]
struct Params {
    channel: String,
    symbol: Vec<String>,
}

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

fn fetch_config() -> Result<AppConfig, config::ConfigError> {
    let config = Config::builder()
        .add_source(config::File::with_name("./config/config.toml").required(true))
        .build()?;

    config.try_deserialize::<AppConfig>()
}

async fn connect(url: String) -> Result<(WsWriter, WsReader)> {
    let (ws_stream, _) = connect_async(&url).await?;
    println!("Handshake successfully completed.");

    Ok(ws_stream.split())
}

async fn ping(write: &mut WsWriter) -> Result<()> {
    let request = PingRequest {
        method: "ping".to_string(),
    };

    let json = serde_json::to_string(&request).expect("Failed to serialize ping message.");
    write.send(Message::Text(json.into())).await?;

    Ok(())
}

async fn subscribe(write: &mut WsWriter, channel: &Channel, symbol: &[String]) -> Result<()> {
    let request = SubscribeRequest {
        method: "subscribe".to_string(),
        params: Params {
            channel: channel.as_str().to_string(),
            symbol: symbol.to_vec(),
        },
    };

    let json = serde_json::to_string(&request).expect("Failed to serialize subscribe message.");
    write.send(Message::Text(json.into())).await?;
    println!("Subscribed to {} channel.", channel.as_str());

    Ok(())
}

async fn subscribe_instrument(write: &mut WsWriter) -> Result<()> {
    let json = serde_json::to_string(&serde_json::json!({
        "method": "subscribe",
        "params": { "channel": Channel::Instruments.as_str() }
    }))
    .expect("Failed to serialize subscribe message.");
    write.send(Message::Text(json.into())).await?;
    println!("Subscribed to {} channel.", Channel::Instruments.as_str());

    Ok(())
}

async fn unsubscribe_instrument(write: &mut WsWriter) -> Result<()> {
    let json = serde_json::to_string(&serde_json::json!({
        "method": "unsubscribe",
        "params": { "channel": Channel::Instruments.as_str() }
    }))
    .expect("Failed to serialize unsubscribe message.");
    write.send(Message::Text(json.into())).await?;
    println!(
        "Unsubscribed from {} channel.",
        Channel::Instruments.as_str()
    );

    Ok(())
}

async fn unsubscribe(write: &mut WsWriter, channel: &Channel, symbol: &[String]) -> Result<()> {
    let request = UnsubscribeRequest {
        method: "unsubscribe".to_string(),
        params: Params {
            channel: channel.as_str().to_string(),
            symbol: symbol.to_vec(),
        },
    };

    let json = serde_json::to_string(&request).expect("Failed to serialize unsubscribe message.");
    write.send(Message::Text(json.into())).await?;
    println!("Unsubscribed from {} channel.", channel.as_str());

    Ok(())
}

async fn process_messages(
    write: &mut WsWriter,
    read: &mut WsReader,
    symbol: &[String],
    limits: &ChannelLimits,
) -> Result<()> {
    let mut ticker_count = 0;
    let mut book_count = 0;
    let mut candle_count = 0;
    let mut trade_count = 0;
    let mut instrument_count = 0;

    while let Some(msg) = read.next().await {
        let msg = msg?;

        if let Message::Text(text) = msg {
            let value: serde_json::Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let msg_method = value.get("method").and_then(|m| m.as_str());
            let msg_type = value.get("type").and_then(|t| t.as_str());
            let msg_channel = value.get("channel").and_then(|c| c.as_str());

            if msg_method == Some("pong") {
                let response: PongResponse =
                    serde_json::from_value(value).expect("Failed to deserialize pong response.");

                println!(
                    "[{}] time_in: {} time_out: {}",
                    response.method, response.time_in, response.time_out
                );
                continue;
            }

            if msg_channel == Some(Channel::Heartbeat.as_str()) {
                continue;
            }

            if msg_channel == Some(Channel::Status.as_str()) {
                let response: StatusUpdateResponse =
                    serde_json::from_value(value).expect("Failed to deserialize status update.");

                println!("[{}][{}]", response.channel, response.tpe);
                for status in &response.data {
                    println!(
                        "  system: {} api: {} version: {} connection_id: {}",
                        status.system, status.api_version, status.version, status.connection_id
                    );
                }
                continue;
            }

            if msg_type != Some("update") {
                continue;
            }

            if msg_channel == Some(Channel::Ticker.as_str()) && ticker_count < limits.ticker {
                let response: TickerUpdateResponse =
                    serde_json::from_value(value).expect("Failed to deserialize ticker update.");

                println!("[{}][{}]", response.channel, response.tpe);
                for tick in &response.data {
                    println!(
                        "  [{}] bid: {} ({}) ask: {} ({}) last: {} vol: {} vwap: {} low: {} high: {} change: {} ({:.2}%)",
                        tick.symbol,
                        tick.bid,
                        tick.bid_qty,
                        tick.ask,
                        tick.ask_qty,
                        tick.last,
                        tick.volume,
                        tick.vwap,
                        tick.low,
                        tick.high,
                        tick.change,
                        tick.change_pct
                    );
                }

                ticker_count += 1;
                println!("Ticker tick {}/{}", ticker_count, limits.ticker);

                if ticker_count >= limits.ticker {
                    unsubscribe(write, &Channel::Ticker, symbol).await?;
                }
            } else if msg_channel == Some(Channel::Book.as_str()) && book_count < limits.book {
                let response: BookUpdateResponse =
                    serde_json::from_value(value).expect("Failed to deserialize book update.");

                println!("[{}][{}]", response.channel, response.tpe);
                for entry in &response.data {
                    println!(
                        "  [{}] checksum: {} timestamp: {}",
                        entry.symbol, entry.checksum, entry.timestamp
                    );
                    for bid in &entry.bids {
                        println!("    bid: {} qty: {}", bid.price, bid.qty);
                    }
                    for ask in &entry.asks {
                        println!("    ask: {} qty: {}", ask.price, ask.qty);
                    }
                }

                book_count += 1;
                println!("Book tick {}/{}", book_count, limits.book);

                if book_count >= limits.book {
                    unsubscribe(write, &Channel::Book, symbol).await?;
                }
            } else if msg_channel == Some(Channel::Candles.as_str()) && candle_count < limits.candle
            {
                let response: CandleUpdateResponse =
                    serde_json::from_value(value).expect("Failed to deserialize candle update.");

                println!(
                    "[{}][{}] {}",
                    response.channel, response.tpe, response.timestamp
                );
                for candle in &response.data {
                    println!(
                        "  [{}] O: {} H: {} L: {} C: {} vwap: {} vol: {} trades: {} interval: {} begin: {} end: {}",
                        candle.symbol,
                        candle.open,
                        candle.high,
                        candle.low,
                        candle.close,
                        candle.vwap,
                        candle.volume,
                        candle.trades,
                        candle.interval,
                        candle.interval_begin,
                        candle.timestamp
                    );
                }

                candle_count += 1;
                println!("Candle tick {}/{}", candle_count, limits.candle);

                if candle_count >= limits.candle {
                    unsubscribe(write, &Channel::Candles, symbol).await?;
                }
            } else if msg_channel == Some(Channel::Trades.as_str()) && trade_count < limits.trade {
                let response: TradeUpdateResponse =
                    serde_json::from_value(value).expect("Failed to deserialize trade update.");

                println!("[{}][{}]", response.channel, response.tpe);
                for trade in &response.data {
                    println!(
                        "  [{}] {} {} @ {} qty: {} type: {} id: {} ts: {}",
                        trade.symbol,
                        trade.side,
                        trade.ord_type,
                        trade.price,
                        trade.qty,
                        trade.ord_type,
                        trade.trade_id,
                        trade.timestamp
                    );
                }

                trade_count += 1;
                println!("Trade tick {}/{}", trade_count, limits.trade);

                if trade_count >= limits.trade {
                    unsubscribe(write, &Channel::Trades, symbol).await?;
                }
            } else if msg_channel == Some(Channel::Instruments.as_str())
                && instrument_count < limits.instrument
            {
                let response: InstrumentUpdateResponse = serde_json::from_value(value)
                    .expect("Failed to deserialize instrument update.");

                println!("[{}][{}]", response.channel, response.tpe);
                println!(
                    "  assets: {} pairs: {}",
                    response.data.assets.len(),
                    response.data.pairs.len()
                );
                for asset in &response.data.assets {
                    println!(
                        "  [asset] {} status: {} precision: {} display: {} borrowable: {} collateral: {} margin_rate: {}",
                        asset.id,
                        asset.status,
                        asset.precision,
                        asset.precision_display,
                        asset.borrowable,
                        asset.collateral_value,
                        asset.margin_rate
                    );
                }
                for pair in &response.data.pairs {
                    println!(
                        "  [pair] {} {}/{} status: {} qty_prec: {} qty_inc: {} price_prec: {} price_inc: {} cost_prec: {} cost_min: {} qty_min: {} marginable: {} has_index: {}",
                        pair.symbol,
                        pair.base,
                        pair.quote,
                        pair.status,
                        pair.qty_precision,
                        pair.qty_increment,
                        pair.price_precision,
                        pair.price_increment,
                        pair.cost_precision,
                        pair.cost_min,
                        pair.qty_min,
                        pair.marginable,
                        pair.has_index
                    );
                    if let Some(margin) = pair.margin_initial {
                        println!(
                            "    margin_initial: {} long_limit: {:?} short_limit: {:?}",
                            margin, pair.position_limit_long, pair.position_limit_short
                        );
                    }
                }

                instrument_count += 1;
                println!("Instrument tick {}/{}", instrument_count, limits.instrument);

                if instrument_count >= limits.instrument {
                    unsubscribe_instrument(write).await?;
                }
            }

            if ticker_count >= limits.ticker
                && book_count >= limits.book
                && candle_count >= limits.candle
                && trade_count >= limits.trade
                && instrument_count >= limits.instrument
            {
                break;
            }
        }
    }

    Ok(())
}
