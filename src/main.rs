use banned_vendor_screen::input::{self, Columns, Preset};
use banned_vendor_screen::lists::Lists;
use banned_vendor_screen::matcher::Index;
use banned_vendor_screen::rules::Funds;
use banned_vendor_screen::{
    build_report, dates, markdown, screen_records, FailOn, InputOut, Settings,
};
use clap::Parser;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

/// Screen payment lines, subawards or award text against DoD 1260H, Section 889, the FCC
/// Covered List, UFLPA and BIOSECURE, with named subsidiaries, GLEIF owners and product names.
///
/// Exit codes: 0 no covered vendor at the --fail-on level; 1 covered vendor found;
/// 2 bad input; 3 lists missing, empty or stale (the screen refuses to pass on them).
#[derive(Parser, Debug)]
#[command(name = "banned-vendor-screen", version, about)]
struct Cli {
    /// CSV, JSON or JSON Lines files to screen.
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Directory holding the us-restricted-parties files (all.jsonl, affiliates.senzing.jsonl,
    /// manifest.json) from https://github.com/Mattbusel/us-restricted-parties/releases/latest
    #[arg(long, env = "BVS_LISTS")]
    lists: PathBuf,

    /// Column layout of the input.
    #[arg(long, value_enum, default_value_t = Preset::Auto)]
    preset: Preset,

    /// Payee / vendor / sub-awardee column.
    #[arg(long)]
    vendor: Option<String>,
    /// Amount column.
    #[arg(long)]
    amount: Option<String>,
    /// Date column (payment, action or start date).
    #[arg(long)]
    date: Option<String>,
    /// Award number column.
    #[arg(long)]
    award: Option<String>,
    /// Grouping column: the institution, department or prime recipient answering for the line.
    #[arg(long)]
    group: Option<String>,
    /// Row id column, carried into the report.
    #[arg(long)]
    id: Option<String>,
    /// Description, title or abstract columns to search for product names (repeatable).
    #[arg(long = "text")]
    text: Vec<String>,
    /// CSV delimiter.
    #[arg(long, default_value = ",")]
    delimiter: char,

    /// Whose money paid for the lines; decides which rules apply.
    #[arg(long, value_enum, default_value_t = Funds::Unknown)]
    funds: Funds,
    /// Lowest match level that fails the run.
    #[arg(long, value_enum, default_value_t = FailOn::Likely)]
    fail_on: FailOn,
    /// Only lines dated on or after this day count toward failing (undated lines still count).
    #[arg(long)]
    since: Option<String>,
    /// Refuse lists older than this many days (manifest.json built_at). 0 skips the check.
    #[arg(long, default_value_t = 8)]
    max_age_days: i64,
    /// Screen names only, without the GLEIF ownership file. Misses owned companies.
    #[arg(long)]
    no_affiliates: bool,

    /// Write the full JSON report here.
    #[arg(long)]
    json: Option<PathBuf>,
    /// Write a Markdown summary here (defaults to $GITHUB_STEP_SUMMARY when set).
    #[arg(long)]
    markdown: Option<PathBuf>,
    /// Rows to show in the Markdown and console tables.
    #[arg(long, default_value_t = 50)]
    max_rows: usize,
    /// Print only the verdict line.
    #[arg(long, short)]
    quiet: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let lists = match Lists::load(&cli.lists, cli.max_age_days, !cli.no_affiliates) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("banned-vendor-screen: {e}");
            return ExitCode::from(3);
        }
    };
    let since = match cli.since.as_deref() {
        None => None,
        Some(s) => match dates::parse_date(s) {
            Some(d) => Some(d),
            None => {
                eprintln!("banned-vendor-screen: --since {s} is not a date (use YYYY-MM-DD)");
                return ExitCode::from(2);
            }
        },
    };
    let settings = Settings {
        fail_on: cli.fail_on,
        since,
        funds: cli.funds,
    };
    let index = Index::new(&lists);
    let mut inputs = Vec::new();
    let mut hits = Vec::new();
    for path in &cli.inputs {
        let table = match input::read(path, cli.delimiter as u8) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("banned-vendor-screen: {e}");
                return ExitCode::from(2);
            }
        };
        let mut cols = Columns::from_preset(cli.preset, &table.headers);
        // Preset columns that this file does not carry are skipped; columns named on the
        // command line must exist.
        let has = |c: &String| table.headers.iter().any(|h| h.eq_ignore_ascii_case(c));
        cols.text.retain(|c| has(c));
        if cols.vendor.as_ref().is_some_and(|c| !has(c)) && cli.vendor.is_none() {
            cols.vendor = None;
        }
        let set = |o: &Option<String>, d: &mut Option<String>| {
            if o.is_some() {
                d.clone_from(o);
            }
        };
        set(&cli.vendor, &mut cols.vendor);
        set(&cli.amount, &mut cols.amount);
        set(&cli.date, &mut cols.date);
        set(&cli.award, &mut cols.award);
        set(&cli.group, &mut cols.group);
        set(&cli.id, &mut cols.id);
        if !cli.text.is_empty() {
            cols.text.clone_from(&cli.text);
        }
        for c in cli.vendor.iter().chain(cli.text.iter()) {
            if !table.headers.iter().any(|h| h.eq_ignore_ascii_case(c)) && !table.records.is_empty()
            {
                eprintln!(
                    "banned-vendor-screen: {}: column \"{c}\" is not in the file (columns: {})",
                    path.display(),
                    table.headers.join(", ")
                );
                return ExitCode::from(2);
            }
        }
        if cols.vendor.is_none() && cols.text.is_empty() {
            eprintln!(
                "banned-vendor-screen: {}: no payee or description column found; pass --vendor or --text (columns: {})",
                path.display(),
                table.headers.join(", ")
            );
            return ExitCode::from(2);
        }
        let name = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        hits.extend(screen_records(
            &index,
            &name,
            &table.records,
            &cols,
            &settings,
        ));
        inputs.push(InputOut {
            file: path.display().to_string(),
            rows: table.records.len(),
            vendor_column: cols.vendor.clone(),
            amount_column: cols.amount.clone(),
            date_column: cols.date.clone(),
            text_columns: cols.text.clone(),
        });
    }
    let report = build_report(&lists, inputs, hits, &settings);
    if let Some(p) = &cli.json {
        let body = serde_json::to_string_pretty(&report).expect("report serializes");
        if let Err(e) = std::fs::write(p, body) {
            eprintln!("banned-vendor-screen: {}: {e}", p.display());
            return ExitCode::from(2);
        }
    }
    let md = markdown(&report, cli.max_rows);
    let md_path = cli
        .markdown
        .clone()
        .or_else(|| std::env::var_os("GITHUB_STEP_SUMMARY").map(PathBuf::from));
    if let Some(p) = md_path {
        let res = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&p)
            .and_then(|mut f| f.write_all(md.as_bytes()));
        if let Err(e) = res {
            eprintln!("banned-vendor-screen: {}: {e}", p.display());
        }
    }
    // GitHub Actions step outputs, so a workflow can branch on the result.
    if let Some(p) = std::env::var_os("GITHUB_OUTPUT") {
        let t = &report.totals;
        let lines = format!(
            "verdict={}\nfailing-rows={}\nfailing-usd={:.2}\nhit-rows={}\nreport={}\n",
            report.verdict,
            t.failing_rows,
            t.failing_usd,
            t.hit_rows,
            cli.json
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        );
        let res = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&p)
            .and_then(|mut f| f.write_all(lines.as_bytes()));
        if let Err(e) = res {
            eprintln!("banned-vendor-screen: GITHUB_OUTPUT: {e}");
        }
    }
    if cli.quiet {
        let t = &report.totals;
        println!(
            "{}: {} failing line(s), {} ({} lines screened)",
            report.verdict,
            t.failing_rows,
            banned_vendor_screen::format_usd(t.failing_usd),
            t.rows
        );
    } else {
        println!("{md}");
    }
    if report.verdict == "fail" {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
