use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Emoji {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Reaction {
    pub count: u32,
    pub me: bool,
    pub emoji: Emoji,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Message {
    RateLimit {
        global: bool,
        message: String,
        retry_after: u32,
    },
    Success {
        id: String,
        channel_id: String,
        guild_id: Option<String>,
        author: User,
        member: Option<GuildMember>,
        content: String,
        timestamp: String,
        edited_timestamp: Option<String>,
        tts: bool,
        reactions: Option<Vec<Reaction>>
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CreateMessageResponse {
    RateLimit {
        global: bool,
        message: String,
        retry_after: u32,
    },
    Success {
        id: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct Channel {
    pub id: String,
    pub r#type: u32,
    pub guild_id: Option<String>,
    pub last_message_id: Option<String>,
    pub name: Option<String>,
    pub topic: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReadyMessage {
    pub v: u32,
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ResumedMessage {
    pub v: u32,
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct HelloMessage {
    pub heartbeat_interval: u32,
}

#[derive(Debug, Deserialize)]
pub struct Guild {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub permissions: Option<u32>,
    pub members: Option<Vec<GuildMember>>,
    pub channels: Option<Vec<Channel>>,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub bot: Option<bool>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GuildMember {
    pub user: Option<User>,
    pub nick: Option<String>,
    pub roles: Vec<String>,
}

#[derive(Debug)]
pub enum DiscordMessage {
    Ready { s: u32, d: ReadyMessage },
    Resumed { s: u32, d: ResumedMessage },
    GuildCreate { s: u32, d: Guild },
    InvalidSession {},
    Hello { d: HelloMessage },
}

impl<'de> serde::Deserialize<'de> for DiscordMessage {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<DiscordMessage, D::Error> {
        struct MessageVisitor;

        use serde::de;

        impl<'de> serde::de::Visitor<'de> for MessageVisitor {
            type Value = DiscordMessage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct DiscordMessage")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut op = None;
                let mut t: Option<Option<String>> = None;
                let mut s: Option<Option<u32>> = None;
                while let Some(key) = map.next_key::<&'de str>()? {
                    match key {
                        "op" => {
                            if op.is_some() {
                                return Err(de::Error::duplicate_field("op"));
                            }
                            op = Some(map.next_value()?);
                        }
                        "t" => {
                            if t.is_some() {
                                return Err(de::Error::duplicate_field("t"));
                            }
                            t = Some(map.next_value()?);
                        }
                        "s" => {
                            if s.is_some() {
                                return Err(de::Error::duplicate_field("s"));
                            }
                            s = Some(map.next_value()?);
                        }
                        "d" => {
                            if !op.is_some() || !s.is_some() || !t.is_some() {
                                return Err(de::Error::custom(
                                    "payload field must come after discriminators",
                                ));
                            }
                            return match (op.unwrap(), t.unwrap()) {
                                (0, Some(t)) => {
                                    if t == "READY" {
                                        Ok(DiscordMessage::Ready {
                                            s: s.unwrap().unwrap(),
                                            d: map.next_value()?,
                                        })
                                    } else if t == "RESUMED" {
                                        Ok(DiscordMessage::Resumed {
                                            s: s.unwrap().unwrap(),
                                            d: map.next_value()?,
                                        })
                                    } else if t == "GUILD_CREATE" {
                                        Ok(DiscordMessage::GuildCreate {
                                            s: s.unwrap().unwrap(),
                                            d: map.next_value()?,
                                        })
                                    } else {
                                        Err(de::Error::unknown_variant(
                                            &format!("{},{:?}", 0, t),
                                            &[],
                                        ))
                                    }
                                }
                                (9, _) => {
                                    map.next_value::<de::IgnoredAny>()?;
                                    Ok(DiscordMessage::InvalidSession {})
                                }
                                (10, _) => Ok(DiscordMessage::Hello {
                                    d: map.next_value()?,
                                }),
                                (op, t) => {
                                    Err(de::Error::unknown_variant(&format!("{},{:?}", op, t), &[]))
                                }
                            };
                        }
                        k => {
                            return Err(de::Error::unknown_field(k, &["op", "t", "s", "d"]));
                        }
                    }
                }
                if !op.is_some() {
                    return Err(de::Error::missing_field("op"));
                }
                if !s.is_some() {
                    return Err(de::Error::missing_field("s"));
                }
                if !t.is_some() {
                    return Err(de::Error::missing_field("t"));
                }
                return Err(de::Error::missing_field("d"));
            }
        }
        d.deserialize_map(MessageVisitor)
    }
}
