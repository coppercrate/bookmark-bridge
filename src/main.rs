use std::env;
use std::fs;
use std::process;

use bookmark_bridge::{merge_bookmarks, parse_bkj, parse_netscape, write_bkj, write_netscape};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "usage: {} <from:netscape|bkj> <to:netscape|bkj> <input-file> [output-file] [--merge <second-input-file>]",
            args.first().map(String::as_str).unwrap_or("bookmark-bridge")
        );
        process::exit(1);
    }

    let from = args[1].as_str();
    let to = args[2].as_str();
    let input_path = &args[3];

    let mut rest: Vec<String> = args[4..].to_vec();
    let merge_path = match rest.iter().position(|a| a == "--merge") {
        Some(pos) => {
            rest.remove(pos);
            if pos >= rest.len() {
                eprintln!("--merge requires a file path");
                process::exit(1);
            }
            Some(rest.remove(pos))
        }
        None => None,
    };
    let output_path = rest.first();

    let input = match fs::read_to_string(input_path) {
        Ok(contents) => contents,
        Err(err) => {
            eprintln!("failed to read {input_path}: {err}");
            process::exit(1);
        }
    };

    let parse = |format: &str, text: &str| match format {
        "netscape" => parse_netscape(text),
        "bkj" => parse_bkj(text),
        other => {
            eprintln!("unknown input format: {other} (expected netscape or bkj)");
            process::exit(1);
        }
    };

    let mut bookmarks = parse(from, &input);

    if let Some(merge_path) = merge_path {
        let merge_input = match fs::read_to_string(&merge_path) {
            Ok(contents) => contents,
            Err(err) => {
                eprintln!("failed to read {merge_path}: {err}");
                process::exit(1);
            }
        };
        let secondary = parse(from, &merge_input);
        bookmarks = merge_bookmarks(&bookmarks, &secondary);
    }

    let output = match to {
        "netscape" => write_netscape(&bookmarks),
        "bkj" => write_bkj(&bookmarks),
        other => {
            eprintln!("unknown output format: {other} (expected netscape or bkj)");
            process::exit(1);
        }
    };

    match output_path {
        Some(path) => {
            if let Err(err) = fs::write(path, output) {
                eprintln!("failed to write {path}: {err}");
                process::exit(1);
            }
        }
        None => print!("{output}"),
    }
}
