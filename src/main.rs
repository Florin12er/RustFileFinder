use chrono::{DateTime, Utc};
use clap::Parser;
use regex::Regex;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Parser, Debug)]
#[clap(
    author = "Apetrei Florin Sebastian",
    version,
    about = "A flexible file finder with content search capabilities",
    long_about = None
)]
struct Args {
    /// Pattern to search for (supports regex and simple glob patterns)
    #[clap(
        short,
        long,
        help = "The search pattern. Supports regex and glob patterns like *.rs"
    )]
    pattern: String,

    /// Directory to start the search from
    #[clap(
        short,
        long,
        default_value = ".",
        help = "The directory to start the search from"
    )]
    dir: String,

    /// Show modification date
    #[clap(long, help = "Display the last modification date of found files")]
    date: bool,

    /// Show file size
    #[clap(short, long, help = "Display the size of found files")]
    size: bool,

    /// Use human-readable file sizes
    #[clap(
        short = 'H',
        long,
        help = "Display file sizes in a human-readable format (KB, MB, GB, etc.)"
    )]
    human_readable: bool,

    /// Sort results (name, size, date)
    #[clap(short = 'S', long, possible_values = &["name", "size", "date"], help = "Sort the results by name, size, or modification date")]
    sort: Option<String>,

    /// Search file contents
    #[clap(
        short = 'c',
        long,
        help = "Search for the pattern within file contents"
    )]
    content_search: bool,
}
#[derive(Debug)]
struct FileInfo {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
    matches_content: bool,
}

fn main() {
    let args = Args::parse();
    let regex_pattern = glob_to_regex(&args.pattern);
    let regex = Regex::new(&regex_pattern).unwrap();
    let mut results = find_files(&args.dir, &regex, &args);

    // Sort results
    if let Some(sort_by) = &args.sort {
        match sort_by.as_str() {
            "name" => results.sort_by(|a, b| a.path.file_name().cmp(&b.path.file_name())),
            "size" => results.sort_by(|a, b| b.size.cmp(&a.size)),
            "date" => results.sort_by(|a, b| b.modified.cmp(&a.modified)),
            _ => {}
        }
    }

    // Display results
    for file in results {
        print!("Found: {:?}", file.path);

        if args.date {
            if let Ok(_) = file.modified.duration_since(SystemTime::UNIX_EPOCH) {
                print!(
                    ", Modified: {}",
                    DateTime::<Utc>::from(file.modified).format("%Y-%m-%d %H:%M:%S")
                );
            }
        }

        if args.size {
            if file.path.is_file() {
                if args.human_readable {
                    print!(", Size: {}", human_readable_size(file.size));
                } else {
                    print!(", Size: {} bytes", file.size);
                }
            }
        }

        if args.content_search && file.matches_content {
            print!(", Matches content");
        }

        println!();
    }
}

fn find_files(dir: &str, regex: &Regex, args: &Args) -> Vec<FileInfo> {
    let mut results = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                let file_name = path.file_name().unwrap().to_string_lossy();

                if regex.is_match(&file_name) || (args.content_search && path.is_file()) {
                    if let Ok(metadata) = fs::metadata(&path) {
                        let matches_content = if args.content_search && path.is_file() {
                            search_file_content(&path, regex)
                        } else {
                            false
                        };

                        if regex.is_match(&file_name) || matches_content {
                            results.push(FileInfo {
                                path: path.clone(),
                                size: metadata.len(),
                                modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                                matches_content,
                            });
                        }
                    }
                }

                if path.is_dir() {
                    results.extend(find_files(path.to_str().unwrap(), regex, args));
                }
            }
        }
    }

    results
}

fn search_file_content(path: &Path, regex: &Regex) -> bool {
    if let Ok(file) = fs::File::open(path) {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                if regex.is_match(&line) {
                    return true;
                }
            }
        }
    }
    false
}

fn glob_to_regex(pattern: &str) -> String {
    let mut regex_pattern = String::new();
    let mut in_brackets = false;

    for c in pattern.chars() {
        match c {
            '*' => {
                if !in_brackets {
                    regex_pattern.push_str(".*");
                } else {
                    regex_pattern.push('*');
                }
            }
            '?' => {
                if !in_brackets {
                    regex_pattern.push('.');
                } else {
                    regex_pattern.push('?');
                }
            }
            '[' => {
                in_brackets = true;
                regex_pattern.push('[');
            }
            ']' => {
                in_brackets = false;
                regex_pattern.push(']');
            }
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '@' | '%' => {
                if !in_brackets {
                    regex_pattern.push('\\');
                }
                regex_pattern.push(c);
            }
            _ => regex_pattern.push(c),
        }
    }

    format!("^{}$", regex_pattern)
}

fn human_readable_size(size: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    if size == 0 {
        return "0 B".to_string();
    }
    let i = (size as f64).log(1024.0).floor() as usize;
    let p = 1024_f64.powi(i as i32);
    let s = (size as f64) / p;
    format!("{:.2} {}", s, UNITS[i])
}
