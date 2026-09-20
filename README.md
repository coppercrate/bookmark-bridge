# bookmark-bridge

Every browser exports bookmarks as an HTML file in the old Netscape
format (`<!DOCTYPE NETSCAPE-Bookmark-file-1>`), a format designed in
1996 and still the only common export target Chrome, Firefox, and
Safari agree on. It's fine for importing back into a browser, but
painful to script against: no schema, folders as nested `<DL>` lists,
dates as raw Unix timestamps in an attribute.

`bookmark-bridge` converts between that HTML format and `bkj`, a
one-JSON-object-per-line format that's easy to grep, diff, and pipe
into other tools.

## Usage

```
bookmark-bridge <from:netscape|bkj> <to:netscape|bkj> <input-file> [output-file]
```

Convert a browser export to `bkj` and print it to stdout:

```
$ bookmark-bridge netscape bkj chrome-bookmarks.html
{"title":"Rust Book","url":"https://doc.rust-lang.org/book/","folder":["Dev"],"added":1700000000}
{"title":"Example","url":"https://example.com","folder":[],"added":null}
```

Convert it back, writing to a file a browser can import:

```
$ bookmark-bridge bkj netscape bookmarks.bkj imported.html
```

## The bkj format

One JSON object per line, four fields, always present:

```json
{"title": "Rust Book", "url": "https://doc.rust-lang.org/book/", "folder": ["Dev"], "added": 1700000000}
```

`folder` is a path from root to the bookmark's containing folder —
`[]` at the top level, `["Parent", "Child"]` when nested. `added` is
`null` when there's no date. There's no top-level array wrapper, so
files can be concatenated, `grep`ped, or streamed line by line.

## Library

The conversion logic lives in `src/lib.rs` as plain functions over
`Bookmark` values and `&str` — no file I/O, no globals:

```rust
pub struct Bookmark {
    pub title: String,
    pub url: String,
    pub folder: Vec<String>,
    pub added: Option<i64>,
}

pub fn parse_netscape(input: &str) -> Vec<Bookmark>;
pub fn write_netscape(bookmarks: &[Bookmark]) -> String;
pub fn parse_bkj(input: &str) -> Vec<Bookmark>;
pub fn write_bkj(bookmarks: &[Bookmark]) -> String;
```

`src/main.rs` is a thin CLI wrapper around these four functions. No
third-party crates — the HTML parsing is a small hand-rolled scanner
and the JSON handling is a minimal encoder/decoder for the flat shape
above, both good enough for the actual shape of bookmark files without
pulling in a general-purpose parser.

## Known limitations

- `write_netscape` groups bookmarks by their `folder` path itself, so
  input order doesn't need to match the output grouping — but a
  folder always appears at the position of the first bookmark that
  used it, and everything with that same path ends up nested inside
  it, even if the input scattered them elsewhere in the slice.

## Building

Standard library only, no dependencies to fetch:

```
cargo build --release
cargo test
```

## License

MIT, see `LICENSE`.
