use crate::discordmessage::*;
use async_native_tls::TlsStream;
use async_std::net::TcpStream;
use futures_util::sink::SinkExt;
use httparse::Header;
use serde_json::json;
use std::collections::HashMap;
use tokio::stream::StreamExt;
use tungstenite::Message;

pub struct DiscordClient {
    raw_tok: String,
    wss: async_tungstenite::WebSocketStream<
        async_tungstenite::stream::Stream<TcpStream, TlsStream<TcpStream>>,
    >,
    pub my_id: String,
    pub session_id: String,
    client: reqwest::Client,
    auth_header: String,
}

impl DiscordClient {
    pub async fn new(tok: String) -> DiscordClient {
        let client = reqwest::Client::builder()
            .user_agent("DiscordBot (https://github.com/ras0219, 0)")
            .build()
            .unwrap();

        let auth_header = format!("Bot {}", tok);

        let mut headers = [Header {
            name: "Authorization",
            value: auth_header.as_bytes(),
        }];
        let mut req = httparse::Request::new(&mut headers);
        req.path = Some(&"wss://gateway.discord.gg/?v=6&encoding=json");
        req.method = Some("GET");
        req.version = Some(b'1');

        let (wss, _) = async_tungstenite::async_std::connect_async(req)
            .await
            .unwrap();

        let mut dclient = DiscordClient {
            my_id: "".to_string(),
            raw_tok: tok,
            wss,
            session_id: "".to_string(),
            client,
            auth_header,
        };
        dclient
    }
    pub async fn create_channel(&mut self, id: &str) -> Channel {
        let mut p = HashMap::new();
        p.insert("recipient_id", id);
        let msg = &self
            .client
            .post("https://discordapp.com/api/v6/users/@me/channels")
            .json(&p)
            .header("Authorization", &self.auth_header)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        println!("create_channel() -> {}", msg);
        serde_json::from_str::<Channel>(msg).unwrap()
    }
    pub async fn create_msg(&mut self, chan_id: &str, content: &str) {
        // let payload = json!({
        //   "content": "Hello, World!",
        //   "tts": false,
        //   "embed": {
        //     "title": "Hello, Embed!",
        //     "description": "This is an embedded message."
        //   }
        // })
        // .to_string();
        for _ in 1..3 {
            let payload = json!({ "content": content }).to_string();
            let res = &self
                .client
                .post(&format!(
                    "https://discordapp.com/api/v6/channels/{}/messages",
                    chan_id
                ))
                .body(payload)
                .header("Authorization", &self.auth_header)
                .header("Content-Type", "application/json")
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            println!("create_msg: {}", res);
            match serde_json::from_str::<RateLimited<CreateMessageResponse>>(res).unwrap() {
                RateLimited::Success { .. } => {
                    break;
                }
                RateLimited::RateLimit { retry_after, .. } => {
                    tokio::time::delay_for(std::time::Duration::from_millis(retry_after.into()))
                        .await;
                }
            }
        }
    }
    pub async fn next_msg(&mut self) -> DiscordMessage {
        loop {
            let msg = self.wss.next().await.unwrap().unwrap();
            if !msg.is_text() {
                continue;
            }
            let dismsg = serde_json::from_str::<DiscordMessage>(&msg.to_string());
            println!("DisMsg: {:?}", dismsg);
            match &dismsg {
                Err(e) => {
                    println!("Msg: {}\n>>> {}", msg, &msg.to_string()[e.column()..]);
                }
                _ => {}
            }
            return dismsg.unwrap();
        }
    }
    pub async fn get_hello(&mut self) {
        self.next_msg().await;
    }
    pub async fn resume(&mut self, session_id: String) {
        let payload = json!({
            "op": 6,
            "d": {
                "token": self.raw_tok,
                "session_id": session_id,
                "seq": 1337
            }
        })
        .to_string();
        self.wss.send(Message::text(payload)).await.unwrap();
        loop {
            let msg = self.next_msg().await;
            match msg {
                DiscordMessage::Resumed { d, .. } => {
                    self.session_id = d.session_id;
                }

                DiscordMessage::InvalidSession {} => {
                    self.identify().await;
                    return;
                }

                _ => {}
            }
        }
    }
    pub async fn identify(&mut self) -> ReadyMessage {
        let payload = json!(
        {
            "op": 2,
            "d": {
              "token": self.raw_tok,
              "properties": {
                "$os": "linux",
                "$browser": "my_library",
                "$device": "my_library"
              }
            }
          })
        .to_string();

        self.wss.send(Message::text(payload)).await.unwrap();
        let msg = self.next_msg().await;
        match msg {
            DiscordMessage::Ready { d, .. } => {
                self.my_id = d.user.id.clone();
                self.session_id = d.session_id.clone();
                d
            }
            _ => panic!(),
        }
    }
    pub async fn get_channel_message(
        &mut self,
        chan: &str,
        msg: &str,
    ) -> crate::discordmessage::Message {
        let msg = &self
            .client
            .get(&format!(
                "https://discordapp.com/api/v6/channels/{}/messages/{}",
                chan, msg
            ))
            .header("Authorization", &self.auth_header)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        println!("get_chan_msg() -> {}", msg);
        serde_json::from_str::<crate::discordmessage::Message>(msg).unwrap()
    }
    pub async fn create_reaction(&mut self, chan: &str, msg: &str, emoji: &str) {
        let msg = &self
            .client
            .put(&format!(
                "https://discordapp.com/api/v6/channels/{}/messages/{}/reactions/{}/@me",
                chan, msg, emoji
            ))
            .header("Content-length", "0")
            .header("Authorization", &self.auth_header)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        println!("create_reaction() -> {}", msg);
    }
}
