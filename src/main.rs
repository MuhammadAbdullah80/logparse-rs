//! `logparse` reads Combined Log Format access logs on stdin (or from files)
//! and prints a rollup: request counts by status class, the slowest paths, and
//! the noisiest clients. It streams line by line so a multi-gigabyte log costs
//! constant memory.

mod parse;
mod stats;

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::process::ExitCode;

use parse::parse_line;
use stats::Summary;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("usage: logparse [FILE...]   (reads stdin when no FILE is given)");
        return ExitCode::SUCCESS;
    }

    let mut summary = Summary::default();
    let result = if args.is_empty() {
        let stdin = io::stdin();
        consume(stdin.lock(), &mut summary)
    } else {
        args.iter().try_for_each(|path| match File::open(path) {
            Ok(f) => consume(BufReader::new(f), &mut summary),
            Err(e) => Err(io::Error::new(e.kind(), format!("{path}: {e}"))),
        })
    };

    if let Err(e) = result {
        eprintln!("logparse: {e}");
        return ExitCode::FAILURE;
    }
    summary.report(&mut io::stdout()).ok();
    ExitCode::SUCCESS
}

/// Feeds every line of `reader` through the parser. Unparseable lines are
/// counted as malformed rather than aborting the run — real logs contain junk.
fn consume<R: Read + BufRead>(reader: R, summary: &mut Summary) -> io::Result<()> {
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match parse_line(&line) {
            Some(entry) => summary.record(&entry),
            None => summary.record_malformed(),
        }
    }
    Ok(())
}
