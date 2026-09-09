use super::{CommandKind, Version};
use crate::{invalid, wire};
use std::{collections::HashSet, io};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Enter,
    Clear,
    CallTab,
    DupeCheck,
}
impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enter => "ENTER",
            Self::Clear => "CLEAR",
            Self::CallTab => "CALLTAB",
            Self::DupeCheck => "DUPECHECK",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parameter {
    Field {
        name: String,
        value: String,
    },
    /// Bare flags such as INCLUDEALL have no closing tag in this protocol.
    Flag(String),
}

/// Complete API command catalog with extensible, ordered parameters.
/// N3FJP's API is tagged text, not XML: values are not entity-escaped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    kind: CommandKind,
    parameters: Vec<Parameter>,
}
impl Command {
    pub fn new(kind: CommandKind) -> Self {
        Self {
            kind,
            parameters: Vec::new(),
        }
    }
    pub fn kind(&self) -> CommandKind {
        self.kind
    }
    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }
    pub fn field(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.push(Parameter::Field {
            name: name.into().to_ascii_uppercase(),
            value: value.into(),
        });
        self
    }
    pub fn flag(mut self, name: impl Into<String>) -> Self {
        self.parameters
            .push(Parameter::Flag(name.into().to_ascii_uppercase()));
        self
    }
    pub fn action(action: Action) -> Self {
        Self::new(CommandKind::Action).field("VALUE", action.as_str())
    }
    pub fn update(control: &str, value: &str) -> Self {
        Self::new(CommandKind::Update)
            .field("CONTROL", control.to_ascii_uppercase())
            .field("VALUE", value)
    }
    pub fn add_adif(record: &str) -> Self {
        Self::new(CommandKind::AddAdifRecord).field("VALUE", record)
    }
    pub fn value(&self, key: &str) -> Option<&str> {
        self.parameters.iter().find_map(|p| match p {
            Parameter::Field { name, value } if name.eq_ignore_ascii_case(key) => {
                Some(value.as_str())
            }
            _ => None,
        })
    }
    pub fn supported_by(&self, version: Version) -> bool {
        version >= self.kind.spec().minimum_version
    }
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut names = HashSet::new();
        let mut body = format!("<CMD><{}>", self.kind.spec().name);
        for parameter in &self.parameters {
            let name = match parameter {
                Parameter::Field { name, .. } | Parameter::Flag(name) => name,
            };
            if !wire::name(name) || matches!(name.as_str(), "CMD") || !names.insert(name) {
                return Err(invalid("invalid or duplicate API parameter"));
            }
            match parameter {
                Parameter::Flag(_) => {
                    if name != "INCLUDEALL"
                        || !matches!(self.kind, CommandKind::List | CommandKind::Search)
                    {
                        return Err(invalid("unsupported bare API flag"));
                    }
                    body.push_str(&format!("<{name}>"));
                }
                Parameter::Field { value, .. } => {
                    if self.kind == CommandKind::AddAdifRecord && name == "VALUE" {
                        validate_adif(value)?;
                    } else {
                        validate_text(value)?;
                    }
                    body.push_str(&format!("<{name}>{value}</{name}>"));
                }
            }
        }
        for required in self.kind.spec().required_fields {
            if self.value(required).is_none() {
                return Err(invalid("missing required API field"));
            }
        }
        if self.kind == CommandKind::Action
            && !matches!(
                self.value("VALUE"),
                Some("ENTER" | "CLEAR" | "CALLTAB" | "DUPECHECK")
            )
        {
            return Err(invalid("unknown ACTION"));
        }
        if matches!(
            self.kind,
            CommandKind::QsoInProgress | CommandKind::UpdateAndLog
        ) && self.value("BAND").unwrap_or_default().is_empty()
            && self.value("FREQ").unwrap_or_default().is_empty()
        {
            return Err(invalid("BAND or FREQ required"));
        }
        if self.kind == CommandKind::Atno
            && self.value("COUNTRYWORKED").is_none()
            && self.value("DXCC").is_none()
        {
            return Err(invalid("COUNTRYWORKED or DXCC required"));
        }
        body.push_str("</CMD>\r\n");
        Ok(body.into_bytes())
    }
}

fn validate_text(value: &str) -> io::Result<()> {
    // The published API does not define a Unicode encoding. Never silently replace characters.
    if !value.is_ascii()
        || value
            .bytes()
            .any(|b| b.is_ascii_control() || b == b'<' || b == b'>')
    {
        return Err(invalid(
            "API text must be printable ASCII without angle brackets",
        ));
    }
    Ok(())
}

/// The API embeds ADIF verbatim. Validate lengths and exactly one record so values
/// cannot escape VALUE/CMD framing. This is transport validation, not QSO scoring.
fn validate_adif(record: &str) -> io::Result<()> {
    if !record.is_ascii() || record.bytes().any(|b| b.is_ascii_control()) {
        return Err(invalid("ADIF must be one ASCII line"));
    }
    let mut rest = record.trim();
    let mut count = 0;
    loop {
        if rest.eq_ignore_ascii_case("<EOR>") && count > 0 {
            return Ok(());
        }
        let tail = rest
            .strip_prefix('<')
            .ok_or_else(|| invalid("invalid ADIF field"))?;
        let (descriptor, after) = tail
            .split_once('>')
            .ok_or_else(|| invalid("invalid ADIF descriptor"))?;
        let parts: Vec<_> = descriptor.split(':').collect();
        if !(2..=3).contains(&parts.len())
            || !wire::name(parts[0])
            || parts
                .get(2)
                .is_some_and(|s| s.len() != 1 || !s.as_bytes()[0].is_ascii_alphabetic())
        {
            return Err(invalid("invalid ADIF descriptor"));
        }
        let length: usize = parts[1]
            .parse()
            .map_err(|_| invalid("invalid ADIF length"))?;
        if length > after.len() {
            return Err(invalid("truncated ADIF value"));
        }
        validate_text(&after[..length])?;
        rest = after[length..].trim_start();
        count += 1;
    }
}
