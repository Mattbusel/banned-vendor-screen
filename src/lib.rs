//! banned-vendor-screen: screen payment lines, subawards or award text against the US
//! banned-vendor lists (DoD 1260H, Section 889, FCC Covered List, UFLPA, BIOSECURE) with
//! owners, named subsidiaries and product names, and fail when a covered vendor is found.

pub mod dates;
pub mod families;
pub mod input;
pub mod lists;
pub mod matcher;
pub mod normalize;
pub mod rules;

use input::{parse_amount, Columns, Record};
use lists::{Entity, Kind, Lists};
use matcher::{How, Index, Level};
use rules::{Funds, RuleStatus};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailOn {
    Confirmed,
    Likely,
    Review,
    Never,
}

impl FailOn {
    pub fn threshold(self) -> Option<Level> {
        match self {
            FailOn::Confirmed => Some(Level::Confirmed),
            FailOn::Likely => Some(Level::Likely),
            FailOn::Review => Some(Level::Review),
            FailOn::Never => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityOut {
    pub record_id: String,
    pub name: String,
    pub list: String,
    pub list_name: String,
    pub listed_on: Option<String>,
    pub parent: Option<String>,
    pub source_url: Option<String>,
    pub affiliate: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchOut {
    pub how: How,
    pub level: Level,
    pub matched_text: String,
    pub note: Option<String>,
    pub entity: EntityOut,
}

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub file: String,
    pub row: usize,
    pub line: Option<u64>,
    pub id: Option<String>,
    pub vendor: Option<String>,
    pub amount: Option<f64>,
    pub date: Option<String>,
    pub award: Option<String>,
    pub group: Option<String>,
    pub level: Level,
    pub counts_toward_fail: bool,
    pub lists: Vec<String>,
    pub chain: Vec<String>,
    pub rules: Vec<RuleStatus>,
    pub matches: Vec<MatchOut>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Tally {
    pub rows: usize,
    pub usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupOut {
    pub group: String,
    pub hit_rows: usize,
    pub hit_usd: f64,
    pub failing_rows: usize,
    pub failing_usd: f64,
    pub vendors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InputOut {
    pub file: String,
    pub rows: usize,
    pub vendor_column: Option<String>,
    pub amount_column: Option<String>,
    pub date_column: Option<String>,
    pub text_columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Totals {
    pub rows: usize,
    pub hit_rows: usize,
    pub confirmed: Tally,
    pub likely: Tally,
    pub review: Tally,
    pub failing_rows: usize,
    pub failing_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub tool: String,
    pub version: String,
    pub verdict: String,
    pub lists_built_at: Option<String>,
    pub list_records: BTreeMap<String, usize>,
    pub affiliates_loaded: usize,
    pub fail_on: FailOn,
    pub since: Option<String>,
    pub funds: Funds,
    pub inputs: Vec<InputOut>,
    pub totals: Totals,
    pub groups: Vec<GroupOut>,
    pub hits: Vec<Hit>,
}

pub struct Settings {
    pub fail_on: FailOn,
    pub since: Option<String>,
    pub funds: Funds,
}

fn entity_out(e: &Entity) -> EntityOut {
    EntityOut {
        record_id: e.record_id.clone(),
        name: e.name.clone(),
        list: e.source.clone(),
        list_name: e.list_name.clone(),
        listed_on: e.listed_on.clone(),
        parent: e.parent.clone(),
        source_url: e.source_url.clone(),
        affiliate: matches!(e.kind, Kind::Affiliate { .. }),
    }
}

fn chain_for(vendor: Option<&str>, m: &matcher::Match, e: &Entity) -> Vec<String> {
    let mut c = Vec::new();
    match m.how {
        How::Product => c.push(format!(
            "line names \"{}\"{}",
            m.matched_text,
            vendor.map(|v| format!(" (payee: {v})")).unwrap_or_default()
        )),
        How::Name => c.push(format!(
            "payee \"{}\" matches a listed name or alias of {} once legal forms are removed",
            m.matched_text, e.name
        )),
        How::Brand => c.push(format!(
            "payee \"{}\" carries the brand of {}",
            m.matched_text, e.name
        )),
        How::BrandWords => c.push(format!(
            "payee \"{}\" contains the brand of {}",
            m.matched_text, e.name
        )),
        How::Affiliate => c.push(format!(
            "payee \"{}\" is {} in GLEIF",
            m.matched_text, e.name
        )),
    }
    match &e.kind {
        Kind::Listed => {
            if let Some(p) = &e.parent {
                c.push(format!("the list names {} as a subsidiary of {p}", e.name));
            }
            c.push(format!(
                "{} is on {}{}",
                e.name,
                e.list_name,
                e.listed_on
                    .as_ref()
                    .map(|d| format!(" (listed {d})"))
                    .unwrap_or_default()
            ));
        }
        Kind::Affiliate {
            owner, verdicts, ..
        } => {
            c.push(format!("GLEIF ownership links tie it to {owner} ({verdicts}); evidence, not a determination"));
        }
    }
    if m.level == Level::Review {
        if let Some(n) = &m.note {
            c.push(format!("review: {n}"));
        }
    }
    c
}

/// Screen one table of records.
pub fn screen_records(
    index: &Index,
    file: &str,
    records: &[Record],
    cols: &Columns,
    settings: &Settings,
) -> Vec<Hit> {
    let threshold = settings.fail_on.threshold();
    let mut hits = Vec::new();
    for r in records {
        let vendor = cols
            .vendor
            .as_deref()
            .and_then(|c| r.get(c))
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let texts: Vec<&str> = cols
            .text
            .iter()
            .filter_map(|c| r.get(c))
            .filter(|s| !s.trim().is_empty())
            .collect();
        let ms = index.screen(vendor, &texts);
        if ms.is_empty() {
            continue;
        }
        let date = cols
            .date
            .as_deref()
            .and_then(|c| r.get(c))
            .and_then(dates::parse_date);
        let amount = cols
            .amount
            .as_deref()
            .and_then(|c| r.get(c))
            .and_then(parse_amount);
        let level = ms.iter().map(|m| m.level).max().unwrap_or(Level::Review);
        let ents: Vec<&Entity> = ms.iter().map(|m| &index.lists.entities[m.entity]).collect();
        let products: Vec<&matcher::Match> = ms.iter().filter(|m| m.how == How::Product).collect();
        let drone = if products.is_empty() {
            None
        } else {
            Some(products.iter().any(|m| m.drone_product))
        };
        let mut lists: Vec<String> = ents
            .iter()
            .flat_map(|e| e.source.split('+').map(str::to_string))
            .collect();
        lists.sort();
        lists.dedup();
        let mut chain = Vec::new();
        for (m, e) in ms.iter().zip(&ents) {
            for step in chain_for(vendor, m, e) {
                if !chain.contains(&step) {
                    chain.push(step);
                }
            }
        }
        let first_listed = ents
            .iter()
            .filter(|e| matches!(e.kind, Kind::Listed))
            .filter_map(|e| e.listed_on.as_deref())
            .min();
        if let (Some(d), Some(l)) = (date.as_deref(), first_listed) {
            if d < l {
                chain.push(format!(
                    "this line ({d}) predates the current listing ({l}): it shows the vendor relationship, not a payment made after listing"
                ));
            }
        }
        let in_window = match (&settings.since, &date) {
            (Some(s), Some(d)) => d >= s,
            _ => true,
        };
        let counts = threshold.is_some_and(|t| level >= t) && in_window;
        hits.push(Hit {
            file: file.to_string(),
            row: r.row,
            line: r.line,
            id: cols
                .id
                .as_deref()
                .and_then(|c| r.get(c))
                .map(str::to_string),
            vendor: vendor.map(str::to_string),
            amount,
            date: date.clone(),
            award: cols
                .award
                .as_deref()
                .and_then(|c| r.get(c))
                .map(str::to_string)
                .filter(|s| !s.is_empty()),
            group: cols
                .group
                .as_deref()
                .and_then(|c| r.get(c))
                .map(str::to_string)
                .filter(|s| !s.is_empty()),
            level,
            counts_toward_fail: counts,
            lists,
            chain,
            rules: rules::rules_for(&ents, drone, date.as_deref(), settings.funds),
            matches: ms
                .iter()
                .zip(&ents)
                .map(|(m, e)| MatchOut {
                    how: m.how,
                    level: m.level,
                    matched_text: m.matched_text.clone(),
                    note: m.note.clone(),
                    entity: entity_out(e),
                })
                .collect(),
        });
    }
    hits
}

pub fn build_report(
    lists: &Lists,
    inputs: Vec<InputOut>,
    hits: Vec<Hit>,
    settings: &Settings,
) -> Report {
    let rows = inputs.iter().map(|i| i.rows).sum();
    let mut t = Totals {
        rows,
        hit_rows: hits.len(),
        confirmed: Tally::default(),
        likely: Tally::default(),
        review: Tally::default(),
        failing_rows: 0,
        failing_usd: 0.0,
    };
    let mut groups: BTreeMap<String, GroupOut> = BTreeMap::new();
    for h in &hits {
        let usd = h.amount.unwrap_or(0.0);
        let tally = match h.level {
            Level::Confirmed => &mut t.confirmed,
            Level::Likely => &mut t.likely,
            Level::Review => &mut t.review,
        };
        tally.rows += 1;
        tally.usd += usd;
        if h.counts_toward_fail {
            t.failing_rows += 1;
            t.failing_usd += usd;
        }
        let key = h.group.clone().unwrap_or_else(|| h.file.clone());
        let g = groups.entry(key.clone()).or_insert_with(|| GroupOut {
            group: key,
            hit_rows: 0,
            hit_usd: 0.0,
            failing_rows: 0,
            failing_usd: 0.0,
            vendors: Vec::new(),
        });
        g.hit_rows += 1;
        g.hit_usd += usd;
        if h.counts_toward_fail {
            g.failing_rows += 1;
            g.failing_usd += usd;
        }
        let v = h
            .vendor
            .clone()
            .unwrap_or_else(|| h.matches[0].matched_text.clone());
        if !g.vendors.contains(&v) {
            g.vendors.push(v);
        }
    }
    let mut groups: Vec<GroupOut> = groups.into_values().collect();
    groups.sort_by(|a, b| {
        b.failing_usd
            .total_cmp(&a.failing_usd)
            .then(b.hit_usd.total_cmp(&a.hit_usd))
    });
    Report {
        tool: "banned-vendor-screen".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        verdict: if t.failing_rows > 0 {
            "fail".into()
        } else {
            "pass".into()
        },
        lists_built_at: lists.built_at.clone(),
        list_records: lists.counts.clone(),
        affiliates_loaded: lists.affiliates,
        fail_on: settings.fail_on,
        since: settings.since.clone(),
        funds: settings.funds,
        inputs,
        totals: t,
        groups,
        hits,
    }
}

/// Format dollars as $1,234.56.
pub fn format_usd(x: f64) -> String {
    let neg = x < 0.0;
    let cents = (x.abs() * 100.0).round() as u64;
    let whole = (cents / 100).to_string();
    let mut s = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            s.push(',');
        }
        s.push(ch);
    }
    format!("{}${}.{:02}", if neg { "-" } else { "" }, s, cents % 100)
}

/// Markdown summary (GitHub step summary, PR comment, or a file).
pub fn markdown(r: &Report, max_rows: usize) -> String {
    let mut s = String::new();
    let t = &r.totals;
    if r.verdict == "fail" {
        s.push_str(&format!(
            "## banned-vendor-screen: FAIL. {} line(s), {}, paid a vendor on a US banned-vendor list\n\n",
            t.failing_rows,
            format_usd(t.failing_usd)
        ));
    } else {
        s.push_str(&format!(
            "## banned-vendor-screen: pass ({} lines screened)\n\n",
            t.rows
        ));
    }
    s.push_str(&format!(
        "Screened {} lines against DoD 1260H, Section 889, the FCC Covered List, UFLPA and BIOSECURE (lists built {}; {} GLEIF affiliates). Confirmed: {} ({}). Likely: {} ({}). Review: {} ({}).\n\n",
        t.rows,
        r.lists_built_at.as_deref().unwrap_or("unknown"),
        r.affiliates_loaded,
        t.confirmed.rows,
        format_usd(t.confirmed.usd),
        t.likely.rows,
        format_usd(t.likely.usd),
        t.review.rows,
        format_usd(t.review.usd)
    ));
    if !r.groups.is_empty() {
        s.push_str("| Payer / group | Failing lines | Failing $ | All hits $ | Vendors |\n|---|---:|---:|---:|---|\n");
        for g in r.groups.iter().take(max_rows) {
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                g.group.replace('|', "/"),
                g.failing_rows,
                format_usd(g.failing_usd),
                format_usd(g.hit_usd),
                g.vendors.join("; ").replace('|', "/")
            ));
        }
        s.push('\n');
    }
    let mut hits: Vec<&Hit> = r.hits.iter().collect();
    hits.sort_by(|a, b| {
        b.level
            .cmp(&a.level)
            .then(b.amount.unwrap_or(0.0).total_cmp(&a.amount.unwrap_or(0.0)))
    });
    if !hits.is_empty() {
        s.push_str("| Level | Row | Date | Payee | Amount | Lists | Why |\n|---|---:|---|---|---:|---|---|\n");
        for h in hits.iter().take(max_rows) {
            s.push_str(&format!(
                "| {} | {}:{} | {} | {} | {} | {} | {} |\n",
                h.level.as_str(),
                h.file,
                h.line
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| format!("#{}", h.row)),
                h.date.as_deref().unwrap_or(""),
                h.vendor.as_deref().unwrap_or("").replace('|', "/"),
                h.amount.map(format_usd).unwrap_or_default(),
                h.lists.join(", "),
                h.chain.join("; ").replace('|', "/")
            ));
        }
        if hits.len() > max_rows {
            s.push_str(&format!(
                "\n{} more in the JSON report.\n",
                hits.len() - max_rows
            ));
        }
    }
    s.push_str("\nA hit is evidence for a reviewer, not a legal determination. Lists: https://github.com/Mattbusel/us-restricted-parties\n");
    s
}
