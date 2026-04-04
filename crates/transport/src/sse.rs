//! Server-Sent Events (SSE) parser for streaming LLM responses.
//!
//! Implements a byte-level SSE parser that correctly handles:
//! - `\n\n` and `\r\n\r\n` frame boundaries
//! - `event:` and `data:` prefixes
//! - Comment lines (starting with `:`)
//! - `[DONE]` termination marker
//! - Partial frames across chunk boundaries

/// Raw SSE frame after parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct SseFrame {
    /// Optional event name from `event:` line.
    pub event_name: Option<String>,
    /// Data payload from `data:` line(s).
    pub data: String,
}

/// Stateful SSE parser. Feed it byte chunks; it yields complete frames.
#[derive(Debug, Default)]
pub struct SseParser {
    buffer: Vec<u8>,
}

impl SseParser {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a chunk of bytes into the parser.
    /// Returns all complete frames parsed from this chunk plus buffered data.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseFrame> {
        self.buffer.extend_from_slice(chunk);
        self.drain_frames()
    }

    /// Signal end of stream. Parses any trailing data as a final frame.
    pub fn finish(&mut self) -> Vec<SseFrame> {
        if self.buffer.is_empty() {
            return Vec::new();
        }
        // Treat remaining buffer as a complete frame by appending a newline
        let trailing = std::mem::take(&mut self.buffer);
        let text = String::from_utf8_lossy(&trailing).into_owned();
        parse_frame_text(&text).into_iter().collect()
    }

    fn drain_frames(&mut self) -> Vec<SseFrame> {
        let mut frames = Vec::new();
        loop {
            // Look for \n\n or \r\n\r\n frame separators
            let separator = find_frame_separator(&self.buffer);
            let Some((end_pos, sep_len)) = separator else {
                break;
            };
            let frame_bytes = self.buffer.drain(..end_pos + sep_len).collect::<Vec<_>>();
            let frame_text = String::from_utf8_lossy(&frame_bytes[..end_pos]).into_owned();
            if let Some(frame) = parse_frame_text(&frame_text) {
                frames.push(frame);
            }
        }
        frames
    }
}

/// Find the position and length of the next frame separator in a byte buffer.
fn find_frame_separator(buf: &[u8]) -> Option<(usize, usize)> {
    // Look for \r\n\r\n first (longer, more specific)
    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
        return Some((pos, 4));
    }
    // Then \n\n
    if let Some(pos) = buf.windows(2).position(|w| w == b"\n\n") {
        return Some((pos, 2));
    }
    None
}

/// Parse a complete frame text into an [`SseFrame`].
/// Returns `None` for empty frames or comment-only frames.
fn parse_frame_text(text: &str) -> Option<SseFrame> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut event_name: Option<String> = None;
    let mut data_lines: Vec<&str> = Vec::new();

    for line in trimmed.lines() {
        if line.starts_with(':') {
            // SSE comment — skip
            continue;
        }
        if let Some(name) = line.strip_prefix("event:") {
            event_name = Some(name.trim().to_string());
        } else if let Some(data) = line.strip_prefix("data:") {
            data_lines.push(data.trim_start_matches(' '));
        } else if line == "data" {
            data_lines.push("");
        }
        // Other fields (id:, retry:) are ignored
    }

    if data_lines.is_empty() {
        return None;
    }

    Some(SseFrame {
        event_name,
        data: data_lines.join("\n"),
    })
}

/// Check if an SSE data field is the `[DONE]` termination sentinel.
#[must_use]
pub fn is_done(data: &str) -> bool {
    data.trim() == "[DONE]"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_data_frame() {
        let mut parser = SseParser::new();
        let frames = parser.push(b"data: hello\n\n");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "hello");
        assert_eq!(frames[0].event_name, None);
    }

    #[test]
    fn parse_named_event() {
        let mut parser = SseParser::new();
        let frames = parser.push(b"event: chunk\ndata: world\n\n");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].event_name.as_deref(), Some("chunk"));
        assert_eq!(frames[0].data, "world");
    }

    #[test]
    fn handles_crlf_separator() {
        let mut parser = SseParser::new();
        let frames = parser.push(b"data: test\r\n\r\n");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "test");
    }

    #[test]
    fn multiple_frames_in_one_chunk() {
        let mut parser = SseParser::new();
        let frames = parser.push(b"data: first\n\ndata: second\n\n");
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].data, "first");
        assert_eq!(frames[1].data, "second");
    }

    #[test]
    fn partial_frame_buffered() {
        let mut parser = SseParser::new();
        let frames1 = parser.push(b"data: hel");
        assert!(frames1.is_empty(), "should buffer partial frame");
        let frames2 = parser.push(b"lo\n\n");
        assert_eq!(frames2.len(), 1);
        assert_eq!(frames2[0].data, "hello");
    }

    #[test]
    fn comments_are_skipped() {
        let mut parser = SseParser::new();
        let frames = parser.push(b": this is a comment\ndata: actual\n\n");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "actual");
    }

    #[test]
    fn done_sentinel() {
        assert!(is_done("[DONE]"));
        assert!(is_done("  [DONE]  "));
        assert!(!is_done("{\"text\": \"hello\"}"));
    }

    #[test]
    fn finish_flushes_trailing() {
        let mut parser = SseParser::new();
        parser.push(b"data: partial");
        let frames = parser.finish();
        // The trailing data without a double-newline should still be parsed
        // (graceful end-of-stream handling)
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "partial");
    }

    #[test]
    fn empty_data_frames_ignored() {
        let mut parser = SseParser::new();
        let frames = parser.push(b"\n\n\n\ndata: real\n\n");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "real");
    }
}
