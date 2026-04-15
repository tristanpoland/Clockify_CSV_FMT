# clockify-processor

A small CLI tool that post-processes [Clockify](https://clockify.me) detailed time report CSV exports. It appends a **Total hours** summary row to each file and renames it to a human-friendly name based on the date range and employee.

```
Clockify_Time_Report_Detailed_01_28_2026-02_10_2026.csv
  →  Hour Report - Tristan Poland - Jan 28 2026 to Feb 10 2026.csv
```

---

## Features

- Processes a **single file** or an entire **directory** in one command
- Appends a `Total hours` row under the decimal hours column
- Renames files using a configurable template with `{name}`, `{start}`, and `{end}` placeholders
- Name can be passed explicitly via `--name` or read automatically from the `User` column in each CSV
- All column names, filename patterns, date formats, and output templates are overridable via flags — no source changes needed if your export format differs

---

## Installation

Requires [Rust](https://rustup.rs) 1.70+.

```bash
git clone <repo>
cd clockify-processor
cargo build --release
```

The binary will be at `target/release/clockify-processor`. Copy it anywhere on your `PATH`.

---

## Usage

```
clockify-processor [OPTIONS] [PATH]
```

`PATH` is a file or directory. If omitted, the current directory is used.

### Examples

```bash
# Process all Clockify CSVs in the current directory
# Name is read from the "User" column in each file
clockify-processor

# Provide a name explicitly
clockify-processor --name "Tristan Poland"

# Point at a specific directory
clockify-processor --name "Tristan Poland" ./reports

# Point at a single file
clockify-processor --name "Tristan Poland" Clockify_Time_Report_Detailed_01_28_2026-02_10_2026.csv

# Custom output filename template
clockify-processor --name "Tristan Poland" --output-template "Invoice - {name} - {start} to {end}"

# Different column names (if your Clockify workspace uses custom fields)
clockify-processor --decimal-col "Hours (decimal)" --user-col "Employee" --label-col "Task"

# Custom date format in the output filename
clockify-processor --date-format "%Y-%m-%d"
# → Hour Report - Tristan Poland - 2026-01-28 to 2026-02-10.csv
```

---

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `PATH` | `.` | File or directory to process |
| `--name`, `-n` | *(from CSV)* | Name embedded in the output filename. Falls back to the `User` column if not provided. |
| `--decimal-col` | `Duration (decimal)` | CSV column to sum for total hours |
| `--user-col` | `User` | CSV column to read the name from (when `--name` is not set) |
| `--label-col` | `Project` | CSV column where the `Total hours` label is placed in the summary row |
| `--output-template` | `Hour Report - {name} - {start} to {end}` | Output filename template. Supports `{name}`, `{start}`, `{end}` |
| `--filename-pattern` | *(see below)* | Regex used to match and parse input filenames |
| `--date-format` | `%b %-d %Y` | [strftime](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) format for dates in the output filename |

### Default filename pattern

```
Clockify_Time_Report_Detailed_(?P<m1>\d{2})_(?P<d1>\d{2})_(?P<y1>\d{4})-(?P<m2>\d{2})_(?P<d2>\d{2})_(?P<y2>\d{4})
```

The pattern must use named capture groups `m1`, `d1`, `y1` (start date) and `m2`, `d2`, `y2` (end date). Override with `--filename-pattern` to adapt to a different export tool.

---

## What the output looks like

Given a Clockify CSV like:

| Project | User | Duration (decimal) | ... |
|---------|------|--------------------|-----|
| Genesis | Tristan Poland | 2.5 | ... |
| R&D | Tristan Poland | 5.0 | ... |

The tool writes a new file with an extra row at the bottom:

| Project | User | Duration (decimal) | ... |
|---------|------|--------------------|-----|
| Genesis | Tristan Poland | 2.5 | ... |
| R&D | Tristan Poland | 5.0 | ... |
| **Total hours** | | **7.5** | |

The original file is left untouched.

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| [`clap`](https://crates.io/crates/clap) | CLI argument parsing |
| [`csv`](https://crates.io/crates/csv) | CSV reading and writing |
| [`chrono`](https://crates.io/crates/chrono) | Date parsing and formatting |
| [`regex`](https://crates.io/crates/regex) | Filename pattern matching |
| [`anyhow`](https://crates.io/crates/anyhow) | Error handling |

---

## Also included

A PowerShell version (`hours.ps1`) is included for use on Windows without needing to install Rust. It supports the same core workflow but with fewer configuration options.

```powershell
# Current directory
.\hours.ps1

# Specific directory or file
.\hours.ps1 -Path "C:\Reports"
.\hours.ps1 -Path "Clockify_Time_Report_Detailed_01_28_2026-02_10_2026.csv"
```