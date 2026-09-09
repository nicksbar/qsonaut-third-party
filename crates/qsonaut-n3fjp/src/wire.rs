use crate::invalid;
use std::io;

pub(crate) fn name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .enumerate()
            .all(|(i, b)| b.is_ascii_alphabetic() || b == b'_' || (i > 0 && b.is_ascii_digit()))
}

pub(crate) fn escape(value: &str) -> io::Result<String> {
    if value
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(invalid("control character in field"));
    }
    Ok(value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;"))
}

pub(crate) fn unescape(value: &str) -> io::Result<String> {
    let mut out = String::new();
    let mut rest = value;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i + 1..];
        let end = rest
            .find(';')
            .ok_or_else(|| invalid("unterminated entity"))?;
        let entity = &rest[..end];
        let ch = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let number = if let Some(hex) = entity.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    entity.strip_prefix('#').and_then(|n| n.parse().ok())
                };
                number
                    .and_then(char::from_u32)
                    .ok_or_else(|| invalid("invalid entity"))?
            }
        };
        out.push(ch);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    escape(&out)?;
    Ok(out)
}

pub(crate) fn tag(key: &str, value: &str) -> io::Result<String> {
    if !name(key) {
        return Err(invalid("invalid field name"));
    }
    Ok(format!("<{key}>{}</{key}>", escape(value)?))
}

/// Read paired leaf tags; tolerate the captured protocol's unclosed containers.
pub(crate) fn leaves(body: &str) -> io::Result<Vec<(String, String)>> {
    let mut fields = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        let end = rest.find('>').ok_or_else(|| invalid("unterminated tag"))?;
        let key = &rest[..end];
        rest = &rest[end + 1..];
        if key.starts_with('/') {
            continue;
        }
        if !name(key) {
            return Err(invalid("invalid tag"));
        }
        let closing = format!("</{key}>");
        if let Some(end) = rest.find(&closing) {
            if !rest[..end].contains('<') {
                fields.push((key.to_owned(), unescape(&rest[..end])?));
                rest = &rest[end + closing.len()..];
            }
        }
    }
    Ok(fields)
}
