use bytes::Bytes;

/// Encode one SSE event exactly like gin's `c.SSEvent` (gin-contrib/sse):
///
///   event: <name>\n
///   data: <line>\n     (one `data:` line per input line)
///   \n
pub fn event(name: &str, data: &str) -> Bytes {
    let mut out = String::new();
    if !name.is_empty() {
        out.push_str("event: ");
        out.push_str(name);
        out.push('\n');
    }
    if !data.is_empty() {
        for line in data.split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            out.push_str("data: ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push('\n');
    Bytes::from(out)
}
