use std::collections::{LinkedList, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

type MessageQueue = Arc<RwLock<VecDeque<String>>>;
type WebsocketPool = Arc<RwLock<LinkedList<Sender<String>>>>;

pub struct BotPool {
    message_queue: MessageQueue,
    websocket_pool: WebsocketPool,
    updater: JoinHandle<()>,
}

impl Default for BotPool {
    fn default() -> Self {
        BotPool::new()
    }
}

impl BotPool {
    pub fn new() -> BotPool {
        let message_queue = MessageQueue::default();
        let websocket_pool = WebsocketPool::default();

        let message_queue_l = message_queue.clone();
        let websocket_pool_l = websocket_pool.clone();

        let updater = tokio::spawn(async move {
            let message_queue = message_queue_l;
            let websocket_pool = websocket_pool_l;

            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;

                let mut websocket_pool = websocket_pool.write().await;
                let mut message_queue = message_queue.write().await;

                match websocket_pool.front_mut().and_then(|fm| message_queue.pop_back().map(|m| (fm, m))) {
                    Some((ws_tx, message)) => {
                        match ws_tx.send(message.clone()).await {
                            Err(_) => {
                                websocket_pool.pop_front();
                                message_queue.push_back(message);
                            }
                            Ok(_) => ()
                        }
                    }
                    _ => ()
                }
            }
        });

        BotPool {
            message_queue,
            websocket_pool,
            updater,
        }
    }

    pub async fn add_websocket(&mut self, ws_tx: Sender<String>) {
        self.websocket_pool.write().await.push_front(ws_tx)
    }

    pub async fn add_message(&mut self, message: String) {
        self.message_queue.write().await.push_front(message)
    }
}



