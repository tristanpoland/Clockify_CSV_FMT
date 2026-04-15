//! clockify-processor
//!
//! Finds Clockify detailed time-report CSVs, appends a "Total hours" row
//! under the numeric duration column, and renames each file to a friendly name.
//!
//! USAGE
//!   clockify-processor [OPTIONS] [PATH]
//!
//!   PATH  File or directory to process (default: current directory)
//!
//! EXAMPLES
//!   clockify-processor
//!   clockify-processor --name "Jane Smith" .
//!   clockify-processor --name "Jane Smith" ./reports
//!   clockify-processor --name "Jane Smith" Clockify_Time_Report_Detailed_01_28_2026-02_10_2026.csv
//!
//!   # Override column names if your Clockify export differs
//!   clockify-processor --decimal-col "Hours (decimal)" --user-col "Employee"
//!
//!   # Override the output filename template
//!   # Placeholders: {name}, {start}, {end}
//!   clockify-processor --output-template "Hours - {name} - {start} to {end}"

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use clap::Parser;
use regex::Regex;

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "clockify-processor",
    about = "Append a total-hours row to Clockify CSV exports and rename them to friendly names"
)]
struct Args {
    /// File or directory to process (default: current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Name to embed in the output filename.
    /// If omitted, the name is read from the "User" column in each CSV.
    #[arg(short, long)]
    name: Option<String>,

    /// CSV column that holds the decimal hours value
    #[arg(long, default_value = "Duration (decimal)")]
    decimal_col: String,

    /// CSV column that holds the user/employee name
    #[arg(long, default_value = "User")]
    user_col: String,

    /// CSV column used for the "Total hours" label in the summary row
    #[arg(long, default_value = "Project")]
    label_col: String,

    /// Output filename template.
    /// Placeholders: {name}, {start}, {end}
    #[arg(long, default_value = "Hour Report - {name} - {start} to {end}")]
    output_template: String,

    /// Regex pattern for matching input filenames.
    /// Must contain exactly 6 named captures: m1 d1 y1 m2 d2 y2 (start/end dates).
    #[arg(
        long,
        default_value = r"Clockify_Time_Report_Detailed_(?P<m1>\d{2})_(?P<d1>\d{2})_(?P<y1>\d{4})-(?P<m2>\d{2})_(?P<d2>\d{2})_(?P<y2>\d{4})"
    )]
    filename_pattern: String,

    /// strftime format for dates in the output filename (default: "Jan 28 2026")
    #[arg(long, default_value = "%b %-d %Y")]
    date_format: String,
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let args = Args::parse();

    let pattern = Regex::new(&args.filename_pattern)
        .context("Invalid --filename-pattern regex")?;

    let files = collect_files(&args.path, &pattern)?;

    if files.is_empty() {
        bail!(
            "No matching CSV files found in: {}",
            args.path.display()
        );
    }

    println!("\nProcessing {} file(s)...\n", files.len());

    for file in &files {
        println!("-> {}", file.display());
        match process_file(file, &args, &pattern) {
            Ok(out) => println!("   [OK] {}", out),
            Err(e)  => eprintln!("   [SKIP] {}", e),
        }
    }

    println!("\nAll done.");
    Ok(())
}

// ── File collection ───────────────────────────────────────────────────────────

fn collect_files(path: &Path, pattern: &Regex) -> Result<Vec<PathBuf>> {
    let meta = fs::metadata(path)
        .with_context(|| format!("Path not found: {}", path.display()))?;

    if meta.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    let mut files: Vec<PathBuf> = fs::read_dir(path)
        .with_context(|| format!("Cannot read directory: {}", path.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(OsStr::to_str) == Some("csv")
                && p.file_stem()
                    .and_then(OsStr::to_str)
                    .map(|s| pattern.is_match(s))
                    .unwrap_or(false)
        })
        .collect();

    files.sort();
    Ok(files)
}

// ── Per-file processing ───────────────────────────────────────────────────────

fn process_file(path: &Path, args: &Args, pattern: &Regex) -> Result<String> {
    let stem = path
        .file_stem()
        .and_then(OsStr::to_str)
        .context("Cannot read filename stem")?;

    // ── Parse dates from filename ─────────────────────────────────────────────
    let caps = pattern
        .captures(stem)
        .with_context(|| format!("Filename does not match pattern: {stem}"))?;

    let start_date = parse_date_caps(&caps, "y1", "m1", "d1")?;
    let end_date   = parse_date_caps(&caps, "y2", "m2", "d2")?;

    let start_str = start_date.format(&args.date_format).to_string();
    let end_str   = end_date.format(&args.date_format).to_string();

    // ── Read CSV ──────────────────────────────────────────────────────────────
    let content = fs::read_to_string(path)
        .with_context(|| format!("Cannot read file: {}", path.display()))?;

    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(content.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .context("Cannot read CSV headers")?
        .iter()
        .map(str::to_string)
        .collect();

    let records: Vec<HashMap<String, String>> = reader
        .records()
        .filter_map(|r| r.ok())
        .map(|r| {
            headers
                .iter()
                .enumerate()
                .map(|(i, h)| (h.clone(), r.get(i).unwrap_or("").to_string()))
                .collect()
        })
        .collect();

    if records.is_empty() {
        bail!("CSV has no data rows");
    }

    // ── Resolve user name ─────────────────────────────────────────────────────
    let user_name = args.name.clone().unwrap_or_else(|| {
        records[0]
            .get(&args.user_col)
            .cloned()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Unknown User".to_string())
    });

    // ── Sum decimal hours ─────────────────────────────────────────────────────
    let total_hours: f64 = records
        .iter()
        .filter_map(|r| {
            r.get(&args.decimal_col)
                .and_then(|v| v.trim().parse::<f64>().ok())
        })
        .sum();

    // ── Build output rows ─────────────────────────────────────────────────────
    // Re-emit all original rows then append the totals row.
    let dir = path.parent().unwrap_or(Path::new("."));

    let friendly_name = args
        .output_template
        .replace("{name}", &user_name)
        .replace("{start}", &start_str)
        .replace("{end}", &end_str);

    let output_path = dir.join(format!("{friendly_name}.csv"));

    let mut writer = csv::WriterBuilder::new()
        .from_path(&output_path)
        .with_context(|| format!("Cannot write: {}", output_path.display()))?;

    // Header
    writer.write_record(&headers)?;

    // Original data rows
    for record in &records {
        let row: Vec<&str> = headers.iter().map(|h| record.get(h).map(String::as_str).unwrap_or("")).collect();
        writer.write_record(&row)?;
    }

    // Total row
    let total_row: Vec<String> = headers
        .iter()
        .map(|h| {
            if h == &args.label_col {
                "Total hours".to_string()
            } else if h == &args.decimal_col {
                total_hours.to_string()
            } else {
                String::new()
            }
        })
        .collect();
    writer.write_record(&total_row)?;
    writer.flush()?;

    Ok(format!(
        "{} ({} hrs)",
        output_path.file_name().unwrap_or_default().to_string_lossy(),
        total_hours
    ))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_date_caps(
    caps: &regex::Captures,
    year_key: &str,
    month_key: &str,
    day_key: &str,
) -> Result<NaiveDate> {
    let y: i32 = caps[year_key].parse()?;
    let m: u32 = caps[month_key].parse()?;
    let d: u32 = caps[day_key].parse()?;
    NaiveDate::from_ymd_opt(y, m, d).with_context(|| format!("Invalid date: {y}-{m:02}-{d:02}"))
}