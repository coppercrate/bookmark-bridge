//! Conversion between the Netscape Bookmark HTML format (what every
//! browser produces via "export bookmarks") and `bkj`, a one-object-
//! per-line JSON format meant to be easy to diff and pipe through
//! other tools.
//!
//! Every public function here is pure: text in, data out, or data in,
//! text out. Nothing touches a filesystem or clock, so the whole
//! surface can be exercised with plain `&str` fixtures.

#[derive(Debug, Clone, PartialEq)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
    /// Folder path, if any. A single string rather than a `Vec` because
    /// the Netscape format only nests folders one level deep in practice
    /// and this keeps round-tripping simple; see the roadmap for nesting.
    pub folder: Option<String>,
    /// Seconds since the Unix epoch, taken from `ADD_DATE` when present.
    pub added: Option<i64>,
}

// ---------------------------------------------------------------------
// Netscape Bookmark HTML
// ---------------------------------------------------------------------

pub fn parse_netscape(input: &str) -> Vec<Bookmark> {
    let mut bookmarks = Vec::new();
    let mut folder_stack: Vec<String> = Vec::new();
    let mut pending_folder: Option<String> = None;

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if let Some(title) = extract_tag_text(line, "H3") {
            pending_folder = Some(title);
        } else if line.starts_with("<DL") {
            if let Some(name) = pending_folder.take() {
                folder_stack.push(name);
            }
        } else if line.starts_with("</DL") {
            folder_stack.pop();
        } else if let Some(bookmark) = parse_anchor_line(line, &folder_stack) {
            bookmarks.push(bookmark);
        }
    }

    bookmarks
}

pub fn write_netscape(bookmarks: &[Bookmark]) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE NETSCAPE-Bookmark-file-1>\n");
    out.push_str("<META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n");
    out.push_str("<TITLE>Bookmarks</TITLE>\n");
    out.push_str("<H1>Bookmarks</H1>\n");
    out.push_str("<DL><p>\n");

    // Assumes bookmarks are already grouped by folder; callers converting
    // from a format with scattered folders should sort first.
    let mut current_folder: Option<&str> = None;
    for bookmark in bookmarks {
        let folder = bookmark.folder.as_deref();
        if folder != current_folder {
            if current_folder.is_some() {
                out.push_str("    </DL><p>\n");
            }
            if let Some(name) = folder {
                out.push_str(&format!("    <DT><H3>{}</H3>\n    <DL><p>\n", escape_html(name)));
            }
            current_folder = folder;
        }

        let indent = if folder.is_some() { "        " } else { "    " };
        let add_date_attr = match bookmark.added {
            Some(ts) => format!(" ADD_DATE=\"{}\"", ts),
            None => String::new(),
        };
        out.push_str(&format!(
            "{indent}<DT><A HREF=\"{}\"{add_date_attr}>{}</A>\n",
            escape_html(&bookmark.url),
            escape_html(&bookmark.title),
        ));
    }
    if current_folder.is_some() {
        out.push_str("    </DL><p>\n");
    }

    out.push_str("</DL><p>\n");
    out
}

fn parse_anchor_line(line: &str, folder_stack: &[String]) -> Option<Bookmark> {
    let (tag, text) = extract_anchor(line)?;
    let url = extract_attr(&tag, "href")?;
    let added = extract_attr(&tag, "add_date").and_then(|s| s.parse::<i64>().ok());
    let folder = if folder_stack.is_empty() {
        None
    } else {
        Some(folder_stack.join("/"))
    };
    Some(Bookmark {
        title: unescape_html(&text),
        url,
        folder,
        added,
    })
}

/// Finds `<A ...>text</A>` in a line and returns the opening tag (for
/// attribute extraction) and the raw inner text.
fn extract_anchor(line: &str) -> Option<(String, String)> {
    let lower = line.to_ascii_lowercase();
    let tag_start = lower.find("<a ")?;
    let tag_end = line[tag_start..].find('>')? + tag_start;
    let tag = line[tag_start..=tag_end].to_string();

    let text_start = tag_end + 1;
    let close = lower[text_start..].find("</a>")? + text_start;
    let text = line[text_start..close].to_string();
    Some((tag, text))
}

/// Returns the unescaped text content of the first `<tag_name>...</tag_name>`
/// found in a line, if any.
fn extract_tag_text(line: &str, tag_name: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let open_needle = format!("<{}", tag_name.to_ascii_lowercase());
    let tag_start = lower.find(&open_needle)?;
    let tag_end = line[tag_start..].find('>')? + tag_start;

    let text_start = tag_end + 1;
    let close_needle = format!("</{}>", tag_name.to_ascii_lowercase());
    let close = lower[text_start..].find(&close_needle)? + text_start;
    Some(unescape_html(&line[text_start..close]))
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{}=\"", attr.to_ascii_lowercase());
    let start = lower.find(&needle)? + needle.len();
    let end = lower[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn unescape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos..];
        let (replacement, consumed_len) = if tail.starts_with("&amp;") {
            ("&", 5)
        } else if tail.starts_with("&lt;") {
            ("<", 4)
        } else if tail.starts_with("&gt;") {
            (">", 4)
        } else if tail.starts_with("&quot;") {
            ("\"", 6)
        } else if tail.starts_with("&#39;") {
            ("'", 5)
        } else if tail.starts_with("&apos;") {
            ("'", 6)
        } else {
            ("&", 1)
        };
        out.push_str(replacement);
        rest = &tail[consumed_len..];
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------------
// bkj: one JSON object per line
// ---------------------------------------------------------------------

pub fn write_bkj(bookmarks: &[Bookmark]) -> String {
    let mut out = String::new();
    for bookmark in bookmarks {
        out.push_str(&bookmark_to_json(bookmark));
        out.push('\n');
    }
    out
}

pub fn parse_bkj(input: &str) -> Vec<Bookmark> {
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| parse_bookmark_json(line.trim()))
        .collect()
}

fn bookmark_to_json(bookmark: &Bookmark) -> String {
    let folder_field = match &bookmark.folder {
        Some(f) => json_string(f),
        None => "null".to_string(),
    };
    let added_field = match bookmark.added {
        Some(ts) => ts.to_string(),
        None => "null".to_string(),
    };
    format!(
        "{{\"title\":{},\"url\":{},\"folder\":{folder_field},\"added\":{added_field}}}",
        json_string(&bookmark.title),
        json_string(&bookmark.url),
    )
}

fn parse_bookmark_json(line: &str) -> Option<Bookmark> {
    let inner = line.strip_prefix('{')?.strip_suffix('}')?;

    let mut title = None;
    let mut url = None;
    let mut folder = None;
    let mut added = None;

    for (key, value) in split_json_fields(inner) {
        match key.as_str() {
            "title" => title = parse_json_string(&value),
            "url" => url = parse_json_string(&value),
            "folder" => folder = if value == "null" { None } else { parse_json_string(&value) },
            "added" => added = if value == "null" { None } else { value.parse::<i64>().ok() },
            _ => {}
        }
    }

    Some(Bookmark {
        title: title?,
        url: url?,
        folder,
        added,
    })
}

/// Splits the inside of a flat `{...}` object into `(key, raw_value)`
/// pairs. Only handles the shapes `write_bkj` produces: string, number,
/// or `null` values, no nested objects or arrays.
fn split_json_fields(inner: &str) -> Vec<(String, String)> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;

    for c in inner.chars() {
        if in_string {
            current.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
            current.push(c);
        } else if c == ',' {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }

    parts
        .into_iter()
        .filter_map(|part| {
            let colon = find_unquoted_colon(&part)?;
            let key = parse_json_string(part[..colon].trim())?;
            let value = part[colon + 1..].trim().to_string();
            Some((key, value))
        })
        .collect()
}

fn find_unquoted_colon(s: &str) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
        } else if c == ':' {
            return Some(i);
        }
    }
    None
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn parse_json_string(s: &str) -> Option<String> {
    let s = s.trim();
    let inner = s.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    let code = u32::from_str_radix(&hex, 16).ok()?;
                    out.push(char::from_u32(code)?);
                }
                other => out.push(other),
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flat_netscape_bookmark() {
        let html = "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n\
                     <DL><p>\n\
                     \x20   <DT><A HREF=\"https://example.com\" ADD_DATE=\"1700000000\">Example</A>\n\
                     </DL><p>\n";
        let bookmarks = parse_netscape(html);
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].title, "Example");
        assert_eq!(bookmarks[0].url, "https://example.com");
        assert_eq!(bookmarks[0].added, Some(1700000000));
        assert_eq!(bookmarks[0].folder, None);
    }

    #[test]
    fn parses_bookmark_inside_folder() {
        let html = "<DL><p>\n\
                     \x20   <DT><H3>Work</H3>\n\
                     \x20   <DL><p>\n\
                     \x20       <DT><A HREF=\"https://example.com\">Example</A>\n\
                     \x20   </DL><p>\n\
                     </DL><p>\n";
        let bookmarks = parse_netscape(html);
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].folder.as_deref(), Some("Work"));
    }

    #[test]
    fn netscape_round_trip_preserves_fields() {
        let original = vec![Bookmark {
            title: "Rust & Friends".to_string(),
            url: "https://rust-lang.org".to_string(),
            folder: Some("Dev".to_string()),
            added: Some(1600000000),
        }];
        let html = write_netscape(&original);
        assert_eq!(parse_netscape(&html), original);
    }

    #[test]
    fn bkj_round_trip_preserves_fields() {
        let original = vec![
            Bookmark {
                title: "No folder".to_string(),
                url: "https://a.example".to_string(),
                folder: None,
                added: None,
            },
            Bookmark {
                title: "Quoted \"title\"".to_string(),
                url: "https://b.example".to_string(),
                folder: Some("Nested/Path".to_string()),
                added: Some(42),
            },
        ];
        let json = write_bkj(&original);
        assert_eq!(parse_bkj(&json), original);
    }

    #[test]
    fn unescape_handles_entities() {
        assert_eq!(unescape_html("Tom &amp; Jerry"), "Tom & Jerry");
        assert_eq!(unescape_html("&lt;tag&gt;"), "<tag>");
        assert_eq!(unescape_html("plain text"), "plain text");
    }
}
