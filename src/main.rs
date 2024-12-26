use chrono::{DateTime, Local};
use clap::Parser;
use colored::*;
use rayon::prelude::*;
use std::fs::File;
use std::io::{self, Write};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Sets the root directory to display
    #[arg(default_value = ".")]
    directory: String,

    /// Sets the maximum depth to traverse
    #[arg(short, long)]
    depth: Option<usize>,

    /// Show hidden files
    #[arg(short, long)]
    all: bool,

    /// Disable colors
    #[arg(short, long)]
    no_color: bool,

    /// Output to a file instead of stdout
    #[arg(short, long)]
    output: Option<String>,
}

fn main() -> io::Result<()> {
    let args: Args = Args::parse();

    let mut output: Box<dyn Write> = match &args.output {
        Some(file_path) => Box::new(File::create(file_path)?),
        None => Box::new(io::stdout()),
    };

    // Create a vector to track the last entry at each depth
    let mut last_dirs: Vec<bool> = Vec::new();

    // Configure WalkDir
    let walker = WalkDir::new(&args.directory)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_hidden(e) || args.all)
        .filter_map(|e| e.ok());

    // Collect entries into a vector
    let entries: Vec<_> = if let Some(max_depth) = args.depth {
        walker
            .filter(|e: &walkdir::DirEntry| e.depth() <= max_depth)
            .collect()
    } else {
        walker.collect()
    };

    for (index, entry) in entries.iter().enumerate() {
        let depth = entry.depth();
        let file_name = entry.file_name().to_string_lossy();

        // Determine if this is the last entry at its depth
        let is_last = {
            let next_index = index + 1;
            if next_index >= entries.len() {
                true
            } else {
                let next_entry = &entries[next_index];
                next_entry.depth() < depth
            }
        };

        // Update last_dirs vector
        if depth >= last_dirs.len() {
            last_dirs.push(is_last);
        } else {
            last_dirs[depth] = is_last;
        }

        // Build the prefix
        let mut prefix = String::new();
        for d in 0..depth {
            if last_dirs.get(d).cloned().unwrap_or(false) {
                prefix.push_str("    ");
            } else {
                prefix.push_str("│   ");
            }
        }

        // Choose the branch character
        if is_last {
            prefix.push_str("└── ");
        } else {
            prefix.push_str("├── ");
        }

        // Determine if the entry is a symbolic link
        let styled_name = if entry.file_type().is_dir() {
            if args.no_color {
                file_name.bold()
            } else {
                file_name.bold().blue()
            }
        } else if entry.file_type().is_symlink() {
            if args.no_color {
                file_name.normal().green()
            } else {
                file_name.normal().green()
            }
        } else {
            file_name.normal()
        };

        // Append symlink target if applicable
        let display_name = if entry.file_type().is_symlink() {
            if let Ok(target) = entry.path().read_link() {
                format!("{} -> {}", styled_name, target.display())
            } else {
                format!("{} -> [unresolved]", styled_name)
            }
        } else {
            styled_name.to_string()
        };

        let metadata = entry.metadata().unwrap();
        let file_size = metadata.len();
        let modified: DateTime<Local> = metadata.modified().unwrap().into();
        let formatted_date = modified.format("%Y-%m-%d %H:%M:%S").to_string();

        // Print the entry
        writeln!(
            output,
            "{}{} ({} bytes, modified: {})",
            prefix, display_name, file_size, formatted_date
        )?;
    }

    Ok(())
}

// Helper function to determine if a file is hidden
fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}
