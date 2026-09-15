use std::env;
use std::fs;
use std::process;

use bookmark_bridge::{parse_bkj, parse_netscape, write_bkj, write_netscape};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "usage: {} <from:netscape|bkj> <to:netscape|bkj> <input-file> [output-file]",
            args.first().map(String::as_str).unwrap_or("bookmark-bridge")
        );
        process::exit(1);
    }

    let from = args[1].as_str();
    let to = args[2].as_str();
    let input_path = &args[3];
    let output_path = args.get(4);

    let input = match fs::read_to_string(input_path) {
        Ok(contents) => contents,
        Err(err) => {
            eprintln!("failed to read {input_path}: {err}");
            process::exit(1);
        }
    };

    let bookmarks = match from {
        "netscape" => parse_netscape(&input),
        "bkj" => parse_bkj(&input),
        other => {
            eprintln!("unknown input format: {other} (expected netscape or bkj)");
            process::exit(1);
        }
    };

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
