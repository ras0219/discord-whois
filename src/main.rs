#![allow(unused_mut)]
use async_native_tls::TlsStream;
use async_std::net::TcpStream;
use futures_util::sink::SinkExt;
use httparse::Header;
use serde_json::json;
use std::collections::HashMap;
use tokio::stream::StreamExt;
use tungstenite::Message;

mod discordmessage;

use crate::discordmessage::*;

struct DiscordClient {
    raw_tok: String,
    wss: async_tungstenite::WebSocketStream<
        async_tungstenite::stream::Stream<TcpStream, TlsStream<TcpStream>>,
    >,
    session_id: String,
    client: reqwest::Client,
    auth_header: String,
}

impl DiscordClient {
    async fn new(tok: String) -> DiscordClient {
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
            raw_tok: tok,
            wss,
            session_id: "".to_string(),
            client,
            auth_header,
        };
        dclient
    }
    async fn create_channel(&mut self, id: &str) -> Channel {
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
    async fn create_msg(&mut self, chan_id: &str, content: &str) {
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
            match serde_json::from_str::<CreateMessageResponse>(res).unwrap() {
                CreateMessageResponse::Success { .. } => {
                    break;
                }
                CreateMessageResponse::RateLimit { retry_after, .. } => {
                    tokio::time::delay_for(std::time::Duration::from_millis(retry_after.into()))
                        .await;
                }
            }
        }
    }
    async fn next_msg(&mut self) -> DiscordMessage {
        let msg = self.wss.next().await.unwrap().unwrap();
        let dismsg = serde_json::from_str::<DiscordMessage>(&msg.to_string());
        println!("DisMsg: {:?}", dismsg);
        if dismsg.is_err() {
            println!("Msg: {}", msg);
        }
        dismsg.unwrap()
    }
    async fn get_hello(&mut self) {
        self.next_msg().await;
    }
    async fn resume(&mut self, session_id: String) {
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
                    return self.identify().await;
                }

                _ => {}
            }
        }
    }
    async fn identify(&mut self) {
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
                self.session_id = d.session_id;
            }
            _ => panic!(),
        }
    }
    async fn get_channel_message(&mut self, chan: &str, msg: &str) -> discordmessage::Message {
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
        serde_json::from_str::<discordmessage::Message>(msg).unwrap()
    }
}

struct DiscordAgent {
    guilds: Vec<Guild>,
}

#[tokio::main]
async fn main() {
    let raw_tok = std::env::var("DISCORD_TOKEN")
        .expect("Expected bot token in DISCORD_TOKEN environment variable");
    let mut dclient = DiscordClient::new(raw_tok).await;
    dclient.get_hello().await;
    let _session_id = match std::env::var("DISCORD_SESSION") {
        Ok(val) => dclient.resume(val).await,
        Err(_) => dclient.identify().await,
    };

    let mut agent = DiscordAgent { guilds: vec![] };

    match dclient.next_msg().await {
        DiscordMessage::GuildCreate { d, .. } => {
            agent.guilds.push(d);
        }
        m => println!("Unknown message: {:?}", m),
    }

    /*|| async {
        for g in &agent.guilds {
            for c in g.channels.as_ref().unwrap() {
                if c.name.as_ref().unwrap() == "bot-playground" {
                    for b in g.members.as_ref().unwrap() {
                        let u = b.user.as_ref().unwrap();
                        if u.bot.unwrap_or(false) {
                            println!("Sending message to {}, {:?}", u.id, u.username);
                            dclient
                                .create_msg(&c.id, &format!("Hello fellow bot <@{}>", u.id))
                                .await;
                        }
                    }
                }
            }
        }
    };*/

    for g in agent.guilds {
        for c in g.channels.as_ref().unwrap() {
            if c.name.as_ref().unwrap() == "bot-playground" {
                for x in &c.last_message_id {
                    println!("Last Message: {:?}", dclient.get_channel_message(&c.id, x).await);
                }
            }
        }
    }

    println!("Hello, world!\n{}", 1);
}
