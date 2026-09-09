use super::Version;
use crate::invalid;
use std::io;

/// Original payload is retained, including nested/repeated records and unknown events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    id: String,
    body: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramInfo {
    pub name: String,
    pub program_version: String,
    pub api_version: Version,
}
impl Packet {
    pub fn parse(body: &str) -> io::Result<Self> {
        let body = body.trim();
        let rest = body
            .strip_prefix('<')
            .ok_or_else(|| invalid("missing response ID"))?;
        let (id, _) = rest
            .split_once('>')
            .ok_or_else(|| invalid("unterminated response ID"))?;
        if id.is_empty()
            || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || id.eq_ignore_ascii_case("CMD")
        {
            return Err(invalid("invalid response ID"));
        }
        if body.to_ascii_uppercase().contains("<CMD>")
            || body.to_ascii_uppercase().contains("</CMD>")
        {
            return Err(invalid("nested API frame"));
        }
        Ok(Self {
            id: id.to_ascii_uppercase(),
            body: body.to_owned(),
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn body(&self) -> &str {
        &self.body
    }
    /// Paired tags are case-insensitive; values remain byte-for-byte unchanged.
    /// This also supports non-XML tag names such as 20MIN and nested record bodies.
    pub fn values(&self, name: &str) -> Vec<&str> {
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Vec::new();
        }
        let start = format!("<{}>", name.to_ascii_uppercase());
        let end = format!("</{}>", name.to_ascii_uppercase());
        let upper = self.body.to_ascii_uppercase();
        let mut offset = 0;
        let mut values = Vec::new();
        while let Some(i) = upper[offset..].find(&start) {
            let begin = offset + i + start.len();
            let Some(j) = upper[begin..].find(&end) else {
                break;
            };
            values.push(&self.body[begin..begin + j]);
            offset = begin + j + end.len();
        }
        values
    }
    pub fn value(&self, name: &str) -> Option<&str> {
        self.values(name).first().copied()
    }
    pub fn program_info(&self) -> io::Result<Option<ProgramInfo>> {
        if self.id != "PROGRAMRESPONSE" {
            return Ok(None);
        }
        let field = |key| {
            self.value(key)
                .ok_or_else(|| invalid("incomplete PROGRAMRESPONSE"))
        };
        Ok(Some(ProgramInfo {
            name: field("PGM")?.into(),
            program_version: field("VER")?.into(),
            api_version: field("APIVER")?.parse()?,
        }))
    }
    /// Only ENTERRESPONSE supplies this count; ENTEREVENT is not an acknowledgment.
    pub fn entered_records(&self) -> io::Result<Option<u32>> {
        if self.id != "ENTERRESPONSE" {
            return Ok(None);
        }
        self.value("VALUE")
            .ok_or_else(|| invalid("missing ENTER count"))?
            .parse()
            .map(Some)
            .map_err(|_| invalid("invalid ENTER count"))
    }
}

/// Incremental CMD framing. TCP reads may split anywhere or coalesce many records.
/// UTF-8 responses are accepted without lossy conversion; outgoing commands are ASCII.
pub struct Decoder {
    buffer: Vec<u8>,
    limit: usize,
}
impl Decoder {
    pub fn new(max_frame_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            limit: max_frame_bytes,
        }
    }
    pub fn pending_bytes(&self) -> usize {
        self.buffer.len()
    }
    pub fn feed(&mut self, bytes: &[u8]) -> io::Result<Vec<Packet>> {
        let result = self.feed_inner(bytes);
        if result.is_err() {
            self.buffer.clear();
        }
        result
    }
    fn feed_inner(&mut self, bytes: &[u8]) -> io::Result<Vec<Packet>> {
        let mut packets = Vec::new();
        for byte in bytes {
            // CRLF separates records but is not required on inbound records.
            if self.buffer.is_empty() && matches!(*byte, b'\r' | b'\n') {
                continue;
            }
            self.buffer.push(*byte);
            if self.buffer.len() > self.limit {
                return Err(invalid("API frame exceeds configured limit"));
            }
            let prefix_length = self.buffer.len().min(5);
            if !self.buffer[..prefix_length].eq_ignore_ascii_case(&b"<CMD>"[..prefix_length]) {
                return Err(invalid("unexpected bytes outside API frame"));
            }
            if self.buffer.ends_with(b"</CMD>")
                || self.buffer.len() >= 6
                    && self.buffer[self.buffer.len() - 6..].eq_ignore_ascii_case(b"</CMD>")
            {
                let body = std::str::from_utf8(&self.buffer[5..self.buffer.len() - 6])
                    .map_err(|_| invalid("response is not valid UTF-8/ASCII"))?;
                packets.push(Packet::parse(body)?);
                self.buffer.clear();
            }
        }
        Ok(packets)
    }
}
