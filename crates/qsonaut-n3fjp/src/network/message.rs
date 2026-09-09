use crate::{invalid, wire};
use std::io;

/// Unknown transaction names and all XMLDATA fields are preserved for contests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transaction {
    Add,
    Update,
    Delete,
    Clear,
    List,
    Check,
    Ack,
    Other(String),
}
impl Transaction {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Add => "ADD",
            Self::Update => "UPDATE",
            Self::Delete => "DELETE",
            Self::Clear => "CLEAR",
            Self::List => "LIST",
            Self::Check => "CHECK",
            Self::Ack => "",
            Self::Other(s) => s,
        }
    }
}
impl From<&str> for Transaction {
    fn from(s: &str) -> Self {
        match s {
            "ADD" => Self::Add,
            "UPDATE" => Self::Update,
            "DELETE" => Self::Delete,
            "CLEAR" => Self::Clear,
            "LIST" => Self::List,
            "CHECK" => Self::Check,
            "" => Self::Ack,
            _ => Self::Other(s.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    Hello(String),
    BandMode {
        station: String,
        band: String,
        mode: String,
    },
    Open,
    Check,
    Who(Vec<String>),
    Chat {
        to: String,
        from: String,
        text: String,
    },
    /// Clock fields are advisory data; this library never sets the system clock.
    Clock(Vec<(String, String)>),
    Transaction {
        from: String,
        kind: Transaction,
        fields: Vec<(String, String)>,
    },
    /// Unrecognized wire body, retained without guessing its semantics.
    Unknown(String),
}

impl Message {
    pub fn body(&self) -> io::Result<String> {
        let tags = |fields: &[(String, String)]| -> io::Result<String> {
            fields.iter().map(|(k, v)| wire::tag(k, v)).collect()
        };
        Ok(match self {
            Self::Hello(s) => wire::tag("HELLO", s)?,
            Self::BandMode {
                station,
                band,
                mode,
            } => format!(
                "<BAMS>{}{}{}</BAMS>",
                wire::tag("STATION", station)?,
                wire::tag("BAND", band)?,
                wire::tag("MODE", mode)?
            ),
            Self::Open => "<NTWK><OPEN></OPEN></NTWK>".into(),
            Self::Check => "<NTWK><CHECK></CHECK></NTWK>".into(),
            Self::Who(stations) => format!(
                "<WHO>{}</WHO>",
                stations
                    .iter()
                    .map(|s| wire::tag("STATION", s))
                    .collect::<io::Result<String>>()?
            ),
            Self::Chat { to, from, text } => format!(
                "<MESG>{}{}{}</MESG>",
                wire::tag("TO", to)?,
                wire::tag("FROM", from)?,
                wire::tag("MSGTXT", text)?
            ),
            Self::Clock(fields) => format!("<SCLK>{}</SCLK>", tags(fields)?),
            Self::Transaction { from, kind, fields } => format!(
                "<NTWK>{}{}<XMLDATA>{}</XMLDATA></NTWK>",
                wire::tag("FROM", from)?,
                wire::tag("TRANSACTION", kind.as_str())?,
                tags(fields)?
            ),
            Self::Unknown(body) => {
                if body.contains("<BOR>") || body.contains("<EOR>") {
                    return Err(invalid("nested frame marker"));
                }
                wire::escape(body)?;
                body.clone()
            }
        })
    }

    pub fn parse(body: &str) -> io::Result<Self> {
        let body = body.trim();
        let fields = wire::leaves(body)?;
        let get = |key: &str| {
            fields
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        };
        let required = |key: &str| get(key).ok_or_else(|| invalid("missing protocol field"));
        Ok(if body.starts_with("<BAMS>") {
            Self::BandMode {
                station: required("STATION")?,
                band: required("BAND")?,
                mode: required("MODE")?,
            }
        } else if body.starts_with("<NTWK>") {
            if let Some(kind) = get("TRANSACTION") {
                let data = if let Some((_, rest)) = body.split_once("<XMLDATA>") {
                    wire::leaves(rest.split("</XMLDATA>").next().unwrap_or(rest))?
                } else {
                    Vec::new()
                };
                Self::Transaction {
                    from: get("FROM").unwrap_or_default(),
                    kind: kind.as_str().into(),
                    fields: data,
                }
            } else if body.contains("<OPEN>") {
                Self::Open
            } else if body.contains("<CHECK>") {
                Self::Check
            } else {
                Self::Unknown(body.into())
            }
        } else if body.starts_with("<WHO>") {
            Self::Who(
                fields
                    .into_iter()
                    .filter(|(k, _)| k == "STATION")
                    .map(|(_, v)| v)
                    .collect(),
            )
        } else if body.starts_with("<MESG>") {
            Self::Chat {
                to: get("TO").unwrap_or_default(),
                from: required("FROM")?,
                text: required("MSGTXT")?,
            }
        } else if body.starts_with("<SCLK>") {
            Self::Clock(fields)
        } else if let Some(greeting) = body.strip_prefix("<HELLO>") {
            // Captured greeting also uses a second opening HELLO as its closer.
            Self::Hello(get("HELLO").unwrap_or(wire::unescape(
                greeting.split('<').next().unwrap_or_default(),
            )?))
        } else {
            Self::Unknown(body.into())
        })
    }
}
