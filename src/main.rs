use std::collections::VecDeque;
use std::io::{BufRead, Read};
use std::net::{IpAddr, SocketAddr};
use std::ops::Not;
use std::string::ToString;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use log::{error, info};
use reqwest::{Method, Request, RequestBuilder, Url};
use reqwest::multipart::{Form, Part};
use tokio::sync::{mpsc, RwLock};
use warp::{Buf, Filter};
use warp::ws::{Message, WebSocket};

use crate::bot_pool::BotPool;

mod bot_pool;

type TelegramMessages = Arc<RwLock<VecDeque<String>>>;
type BotPoolArc = Arc<RwLock<BotPool>>;

struct Config {
    secret_token: &'static str,
    bot_token: &'static str,
    cert_path: &'static str,
    key_path: &'static str,
    port: u16,
    local_url: String,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let config = Config {
        secret_token: "whatever secret token you'd like to use here",
        bot_token: "bot:token-here",
        cert_path: "certs/cert.pem",
        key_path: "certs/key.pem",
        port: 443,
        local_url: "your.ip.address.here".to_string(),
    };

    yell_at_telegram(&config).await;

    let pool = BotPoolArc::default();
    let warp_pool = warp::any().map(move || pool.clone());

    let telegram_ep = warp::path("tg_update")
        .and(warp::post())
        .and(warp::body::aggregate())
        .and(warp::header::exact("X-Telegram-Bot-Api-Secret-Token", &config.secret_token))
        .and(warp_pool.clone())
        .then(|body, pool| async move {
            read_body_add_to_queue(body, pool).await;
            warp::reply()
        });

    let ws_connect = warp::path("ws_connect")
        .and(warp::ws())
        .and(warp_pool)
        .map(move |ws: warp::ws::Ws, pool| {
            ws.on_upgrade(move |socket| ws_proxy_connected(socket, pool))
        });

    let routes = telegram_ep.or(ws_connect);

    let address: SocketAddr = SocketAddr::new(IpAddr::from([0, 0, 0, 0]), config.port.clone());

    warp::serve(routes).tls()
        .cert_path(&config.cert_path)
        .key_path(&config.key_path)
        .run(address).await;
}

async fn yell_at_telegram(config: &Config) {
    let mut ip = config.local_url.clone();

    ip.push_str(":");
    ip.push_str(config.port.to_string().as_str());
    ip.push_str("/tg_update");

    let cert = tokio::fs::read_to_string(&config.cert_path).await.expect("can't read cert file");
    let cert = Part::stream(cert).file_name("certificate").mime_str("text/plain").expect("can't mime file???");

    let mut url = "https://api.telegram.org/bot".to_owned();
    url.push_str(&config.bot_token);
    url.push_str("/setWebhook");
    let url = Url::parse(&url).expect("can't parse url");

    let client = reqwest::Client::new();

    let request = Request::new(Method::POST, url);
    let request = RequestBuilder::from_parts(client, request);
    let request = request.multipart(Form::new().part("certificate", cert));
    let request = request.query(&[("secret_token", &config.secret_token), ("url", &ip.to_string().as_str())]);

    let response = request.send().await.expect("couldn't update telegram hook ip");
}

async fn read_body_add_to_queue(mut body: impl Buf, pool: BotPoolArc) {
    let mut s = String::new();

    while body.has_remaining() {
        let chunk = body.chunk();

        s += std::str::from_utf8(chunk).expect("not a string???");

        let len = chunk.len();
        body.advance(len);
    }

    pool.write().await.add_message(s).await
}


async fn ws_proxy_connected(ws: WebSocket, pool: BotPoolArc) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (tx, mut rx) = mpsc::channel(1);

    pool.write().await.add_websocket(tx).await;

    let x = tokio::task::spawn(async move {
        while let Some(message) = rx.recv().await {
            ws_tx.send(Message::text(message))
                .unwrap_or_else(|e| {
                    error!("websocket send error: {}", e)
                })
                .await;
        }
    });

    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(v) =>
                info!("got message from ws {}", v.to_str().unwrap_or("[weird value]")),
            Err(e) => {
                error!("got error while reading from ws {}", e);
                break;
            }
        }
    }

    x.abort()
}