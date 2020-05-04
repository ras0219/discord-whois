use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
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
pub enum RateLimited<T> {
    RateLimit {
        global: bool,
        message: String,
        retry_after: u32,
    },
    Success {
        #[serde(flatten)]
        data: T,
    },
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub guild_id: Option<String>,
    pub author: User,
    pub member: Option<GuildMember>,
    pub content: String,
    pub timestamp: String,
    pub edited_timestamp: Option<String>,
    pub tts: bool,
    pub reactions: Option<Vec<Reaction>>,
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

#[derive(Debug, Deserialize, Clone)]
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
    pub user: User,
    pub session_id: String,
    pub guilds: Vec<UnavailableGuild>,
}

#[derive(Debug, Deserialize)]
pub struct ResumedMessage {
    pub v: u32,
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct HelloMessage {
    pub heartbeat_interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct UnavailableGuild {
    pub id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Guild {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub permissions: Option<u32>,
    pub members: Option<Vec<GuildMember>>,
    pub channels: Option<Vec<Channel>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct User {
    pub id: String,
    pub username: Option<String>,
    pub discriminator: Option<String>,
    pub bot: Option<bool>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GuildMember {
    pub user: Option<User>,
    pub nick: Option<String>,
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Activity {
    pub name: String,
    pub r#type: u32,
    pub created_at: u64,
    pub application_id: Option<String>,
    pub details: Option<String>,
    pub state: Option<String>,
    pub emoji: Option<Emoji>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClientStatus {
    pub web: Option<String>,
    pub desktop: Option<String>,
    pub mobile: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PresenceUpdate {
    pub user: User,
    pub game: Option<Activity>,
    pub guild_id: String,
    pub client_status: ClientStatus,
    pub nick: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TypingStart {
    pub user_id: String,
    pub channel_id: String,
    pub timestamp: u64,
}

#[derive(Debug)]
pub enum DiscordMessage {
    Ready {
        s: u64,
        d: ReadyMessage,
    },
    Resumed {
        s: u64,
        d: ResumedMessage,
    },
    GuildCreate {
        s: u64,
        d: Guild,
    },
    PresenceUpdate {
        s: u64,
        d: PresenceUpdate,
    },
    MessageCreate {
        s: u64,
        d: Message,
    },
    Unknown {
        s: u64,
        t: String,
        d: serde_json::Value,
    },
    InvalidSession {},
    HeartbeatAck {},
    Hello {
        d: HelloMessage,
    },
}

impl DiscordMessage {
    pub fn seq(&self) -> Option<u64> {
        match &self {
            Self::Ready { s, .. } => Some(*s),
            Self::Resumed { s, .. } => Some(*s),
            Self::GuildCreate { s, .. } => Some(*s),
            Self::PresenceUpdate { s, .. } => Some(*s),
            Self::MessageCreate { s, .. } => Some(*s),
            Self::Unknown { s, .. } => Some(*s),
            Self::InvalidSession {} => None,
            Self::HeartbeatAck {} => None,
            Self::Hello { .. } => None,
        }
    }
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
                let mut s: Option<Option<u64>> = None;
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
                                    } else if t == "PRESENCE_UPDATE" {
                                        Ok(DiscordMessage::PresenceUpdate {
                                            s: s.unwrap().unwrap(),
                                            d: map.next_value()?,
                                        })
                                    } else if t == "MESSAGE_CREATE" {
                                        Ok(DiscordMessage::MessageCreate {
                                            s: s.unwrap().unwrap(),
                                            d: map.next_value()?,
                                        })
                                    } else {
                                        Ok(DiscordMessage::Unknown {
                                            t,
                                            s: s.unwrap().unwrap(),
                                            d: map.next_value()?,
                                        })
                                    }
                                }
                                (9, _) => {
                                    map.next_value::<de::IgnoredAny>()?;
                                    Ok(DiscordMessage::InvalidSession {})
                                }
                                (10, _) => Ok(DiscordMessage::Hello {
                                    d: map.next_value()?,
                                }),
                                (11, _) => {
                                    map.next_value::<de::IgnoredAny>()?;
                                    Ok(DiscordMessage::HeartbeatAck {})
                                }
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
