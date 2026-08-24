//! Parser for the Combined Log Format:
//! `host - - [ts] "METHOD /path HTTP/1.1" status bytes "referer" "agent" duration`
//!
//! The trailing duration field is an nginx `$request_time` extension; it is
//! optional, so plain Apache/CLF lines parse too.

/// One successfully parsed access-log line.
#[derive(Debug, PartialEq)]
pub struct Entry {
    pub host: String,
    pub path: String,
    pub status: u16,
    pub bytes: u64,
    /// Request duration in milliseconds, when the log carries one.
    pub duration_ms: Option<u64>,
}

/// Parses one line, returning `None` if it does not match the expected shape.
pub fn parse_line(line: &str) -> Option<Entry> {
    let host = line.split_whitespace().next()?.to_string();

    // The request target lives inside the first quoted section.
    let start = line.find('"')?;
    let rest = &line[start + 1..];
    let end = rest.find('"')?;
    let request = &rest[..end];
    let path = request.split_whitespace().nth(1)?.to_string();

    // Status and byte count are the two fields directly after the closing quote.
    let tail = &rest[end + 1..];
    let mut fields = tail.split_whitespace();
    let status: u16 = fields.next()?.parse().ok()?;
    let bytes: u64 = match fields.next()? {
        "-" => 0,
        n => n.parse().ok()?,
    };

    Some(Entry {
        host,
        path,
        status,
        bytes,
        duration_ms: trailing_duration(tail),
    })
}

/// Reads the optional trailing `$request_time` (seconds, e.g. `0.084`) and
/// converts it to whole milliseconds.
fn trailing_duration(tail: &str) -> Option<u64> {
    let last = tail.split_whitespace().last()?;
    let seconds: f64 = last.parse().ok()?;
    if seconds.is_finite() && seconds >= 0.0 {
        Some((seconds * 1000.0).round() as u64)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = r#"10.0.0.7 - - [11/Aug/2026:09:14:02 +0000] "GET /api/users HTTP/1.1" 200 1234 "-" "curl/8.4.0" 0.084"#;

    #[test]
    fn parses_a_full_combined_line() {
        let e = parse_line(LINE).expect("should parse");
        assert_eq!(e.host, "10.0.0.7");
        assert_eq!(e.path, "/api/users");
        assert_eq!(e.status, 200);
        assert_eq!(e.bytes, 1234);
        assert_eq!(e.duration_ms, Some(84));
    }

    #[test]
    fn treats_dash_byte_count_as_zero() {
        let line = r#"1.2.3.4 - - [x] "POST /login HTTP/1.1" 302 -"#;
        assert_eq!(parse_line(line).unwrap().bytes, 0);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_line("not a log line at all").is_none());
        assert!(parse_line("").is_none());
    }
}
