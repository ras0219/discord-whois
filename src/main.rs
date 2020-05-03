#![allow(unused_mut)]
use serde::{Deserialize, Serialize};

mod discordclient;
mod discordmessage;

use crate::discordclient::*;
use crate::discordmessage::*;

#[derive(Deserialize, Serialize, Debug)]
struct DiscordAgentState {
    #[serde(skip)]
    dirty: bool,
    userlist: std::collections::HashMap<String, i32>,
}

impl DiscordAgentState {
    fn new() -> Self {
        DiscordAgentState {
            dirty: false,
            userlist: std::collections::HashMap::new(),
        }
    }
    fn from_file(filename: &str) -> Self {
        let file = std::fs::File::open(filename);
        match file {
            Ok(f) => serde_json::de::from_reader(std::io::BufReader::new(f)).unwrap(),
            _ => Self::new(),
        }
    }
    fn to_file_if_dirty(&mut self, filename: &str) {
        if self.dirty {
            self.dirty = false;
            self.to_file(filename);
        }
    }
    fn to_file(&self, filename: &str) {
        let file = std::fs::File::create(filename).unwrap();
        serde_json::ser::to_writer_pretty(std::io::BufWriter::new(file), self).unwrap();
    }
}

struct DiscordAgent<'a> {
    dclient: &'a mut DiscordClient,
    promised_guilds: usize,
    guilds: Vec<Guild>,
    exit: bool,
    state: DiscordAgentState,
}

impl<'a> DiscordAgent<'a> {
    fn new(dclient: &'a mut DiscordClient) -> Self {
        Self {
            promised_guilds: 0,
            guilds: vec![],
            exit: false,
            dclient,
            state: DiscordAgentState::new(),
        }
    }

    async fn on_msg(&mut self, msg: &DiscordMessage) {
        match msg {
            DiscordMessage::GuildCreate { d, .. } => {
                self.guilds.push(d.clone());
                if self.guilds.len() == self.promised_guilds {
                    self.on_all_guilds().await;
                }
            }
            DiscordMessage::MessageCreate { d: msg, .. } => {
                if msg.author.id == self.dclient.my_id {
                    return;
                }
                if msg.content.starts_with("%say ") {
                    self.dclient
                        .create_msg(&msg.channel_id, &msg.content[5..])
                        .await;
                } else if msg.content.starts_with("++") {
                    let counter = self
                        .state
                        .userlist
                        .entry(msg.content[2..].to_string())
                        .or_insert(0);
                    *counter += 1;
                    self.state.dirty = true;
                    self.dclient
                        .create_reaction(&msg.channel_id, &msg.id, "%f0%9f%8d%80")
                        .await;
                } else if msg.content.ends_with("++") {
                    let counter = self
                        .state
                        .userlist
                        .entry(msg.content[..(msg.content.len() - 2)].to_string())
                        .or_insert(0);
                    *counter += 1;
                    self.state.dirty = true;
                    self.dclient
                        .create_reaction(&msg.channel_id, &msg.id, "%f0%9f%8d%80")
                        .await;
                } else if msg.content.starts_with("%karma ") {
                    let value = self
                        .state
                        .userlist
                        .get(&msg.content[7..].to_string())
                        .unwrap_or(&0);
                    self.dclient
                        .create_msg(&msg.channel_id, &format!("Karma: {}", value))
                        .await;
                }
            }
            _ => {}
        }
        self.state.to_file_if_dirty("data.json");
    }

    async fn on_all_guilds(&mut self) {
        for g in &self.guilds {
            for c in g.channels.as_ref().unwrap() {
                if c.name.as_ref().unwrap() == "bot-playground" {
                    for x in &c.last_message_id {
                        let lastmsg = self.dclient.get_channel_message(&c.id, x).await;
                        println!("Last Message: {:?}", lastmsg);
                        self.dclient.create_reaction(&c.id, x, "%f0%9f%94%a5").await;
                    }
                }
            }
        }
    }

    async fn main_loop(&mut self) {
        while !self.exit {
            let msg = self.dclient.next_msg().await;
            self.on_msg(&msg).await;
        }
    }
}

#[tokio::main]
async fn main() {
    let raw_tok = std::env::var("DISCORD_TOKEN")
        .expect("Expected bot token in DISCORD_TOKEN environment variable");
    let mut dclient = DiscordClient::new(raw_tok).await;
    dclient.get_hello().await;
    // let ready = match std::env::var("DISCORD_SESSION") {
    //     Ok(val) => dclient.resume(val).await,
    //     Err(_) => dclient.identify().await,
    // };
    let ready = dclient.identify().await;

    let mut agent = DiscordAgent::new(&mut dclient);
    agent.promised_guilds = ready.guilds.len();
    agent.state = DiscordAgentState::from_file("data.json");
    agent.main_loop().await;

    println!("Terminating successfully");
}
