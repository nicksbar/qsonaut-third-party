use super::Message;
use crate::invalid;
use std::io;

pub fn encode(message: &Message) -> io::Result<Vec<u8>> {
    Ok(format!("<BOR>{}<EOR>\u{3}\u{4}\u{7}", message.body()?)
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect())
}

/// Byte-oriented framing tolerates odd TCP splits and both observed trailer forms.
/// An error clears the buffer; callers should disconnect on malformed input.
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
    pub fn feed(&mut self, bytes: &[u8]) -> io::Result<Vec<Message>> {
        let mut messages = Vec::new();
        // Bound memory even when a caller supplies a very large input slice.
        for byte in bytes {
            self.buffer.push(*byte);
            if self.buffer.len() > self.limit {
                self.buffer.clear();
                return Err(invalid("N3FJP frame exceeds configured limit"));
            }
            if !self.buffer.ends_with(b"<\0E\0O\0R\0>\0")
                && !self.buffer.ends_with(b"<\0B\0O\0R\0>\0")
            {
                continue;
            }
            if let Err(error) = self.extract(&mut messages) {
                self.buffer.clear();
                return Err(error);
            }
        }
        Ok(messages)
    }
    fn extract(&mut self, out: &mut Vec<Message>) -> io::Result<()> {
        const BOR: &[u8] = b"<\0B\0O\0R\0>\0";
        const EOR: &[u8] = b"<\0E\0O\0R\0>\0";
        let find =
            |bytes: &[u8], marker: &[u8]| bytes.windows(marker.len()).position(|s| s == marker);
        let Some(start) = find(&self.buffer, BOR) else {
            return Ok(());
        };
        if start > 0 {
            self.buffer.drain(..start);
        }
        let tail = &self.buffer[BOR.len()..];
        let end = find(tail, EOR).map(|i| (i, EOR.len()));
        let next = find(tail, BOR).map(|i| (i, 0));
        let Some((end, skip)) = end.into_iter().chain(next).min_by_key(|(i, _)| *i) else {
            return Ok(());
        };
        let payload = &tail[..end];
        if !payload.len().is_multiple_of(2) {
            return Err(invalid("odd UTF-16 frame length"));
        }
        let units: Vec<_> = payload
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        let body = String::from_utf16(&units).map_err(|_| invalid("invalid UTF-16 frame"))?;
        out.push(Message::parse(&body)?);
        self.buffer.drain(..BOR.len() + end + skip);
        Ok(())
    }
}
