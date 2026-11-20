//! Reads payment lines, subawards or awards from CSV, JSON or JSON Lines. Nested JSON objects
//! are flattened to dotted keys (`organization.org_name`), so NIH RePORTER and USAspending API
//! output loads as it comes.

use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Record {
    /// 1-based data row (header excluded for CSV; array index + 1 for JSON).
    pub row: usize,
    /// 1-based line in the file where the row starts (CSV and JSONL); None for a JSON array.
    pub line: Option<u64>,
    pub fields: Vec<(String, String)>,
}

impl Record {
    pub fn get(&self, col: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == col)
            .or_else(|| {
                self.fields
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(col))
            })
            .map(|(_, v)| v.as_str())
    }
}

pub struct Table {
    pub headers: Vec<String>,
    pub records: Vec<Record>,
}

fn flatten(prefix: &str, v: &Value, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&key, x, out);
            }
        }
        Value::Array(a) => {
            let scalars: Vec<String> = a
                .iter()
                .filter_map(|x| match x {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .collect();
            if !scalars.is_empty() {
                out.push((prefix.to_string(), scalars.join("; ")));
            }
        }
        Value::Null => out.push((prefix.to_string(), String::new())),
        Value::String(s) => out.push((prefix.to_string(), s.clone())),
        other => out.push((prefix.to_string(), other.to_string())),
    }
}

fn headers_of(records: &[Record]) -> Vec<String> {
    let mut h: Vec<String> = Vec::new();
    for r in records {
        for (k, _) in &r.fields {
            if !h.contains(k) {
                h.push(k.clone());
            }
        }
    }
    h
}

pub fn read(path: &Path, delimiter: u8) -> Result<Table, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let text = text.trim_start_matches('\u{feff}');
    let trimmed = text.trim_start();
    if ext == "jsonl" || ext == "ndjson" {
        let mut records = Vec::new();
        for (i, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let v: Value = serde_json::from_str(line)
                .map_err(|e| format!("{} line {}: {e}", path.display(), i + 1))?;
            let mut fields = Vec::new();
            flatten("", &v, &mut fields);
            records.push(Record {
                row: records.len() + 1,
                line: Some(i as u64 + 1),
                fields,
            });
        }
        return Ok(Table {
            headers: headers_of(&records),
            records,
        });
    }
    if ext == "json" || trimmed.starts_with('[') || trimmed.starts_with('{') {
        let v: Value =
            serde_json::from_str(text).map_err(|e| format!("{}: {e}", path.display()))?;
        let arr = match &v {
            Value::Array(a) => a.clone(),
            Value::Object(m) => ["results", "data", "rows", "items"]
                .iter()
                .find_map(|k| m.get(*k).and_then(Value::as_array).cloned())
                .ok_or_else(|| {
                    format!(
                        "{}: JSON object has no results/data/rows/items array",
                        path.display()
                    )
                })?,
            _ => {
                return Err(format!(
                    "{}: expected a JSON array of objects",
                    path.display()
                ))
            }
        };
        let records: Vec<Record> = arr
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let mut fields = Vec::new();
                flatten("", x, &mut fields);
                Record {
                    row: i + 1,
                    line: None,
                    fields,
                }
            })
            .collect();
        return Ok(Table {
            headers: headers_of(&records),
            records,
        });
    }
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(text.as_bytes());
    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| format!("{}: {e}", path.display()))?
        .iter()
        .map(|h| h.trim().to_string())
        .collect();
    let mut records = Vec::new();
    for (i, rec) in rdr.records().enumerate() {
        let rec = rec.map_err(|e| format!("{}: {e}", path.display()))?;
        let line = rec.position().map(|p| p.line());
        let fields = headers
            .iter()
            .cloned()
            .zip(rec.iter().map(str::to_string))
            .collect();
        records.push(Record {
            row: i + 1,
            line,
            fields,
        });
    }
    Ok(Table { headers, records })
}

/// Parse "$1,234.50", "(12.00)" or "-12" into a number.
pub fn parse_amount(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let neg = t.starts_with('(') && t.ends_with(')') || t.starts_with('-');
    let digits: String = t
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let v: f64 = digits.parse().ok()?;
    Some(if neg { -v } else { v })
}

#[derive(Debug, Clone, Default)]
pub struct Columns {
    pub vendor: Option<String>,
    pub amount: Option<String>,
    pub date: Option<String>,
    pub award: Option<String>,
    pub group: Option<String>,
    pub id: Option<String>,
    pub text: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Preset {
    /// Guess columns from common header names.
    Auto,
    /// USAspending spending_by_award with subawards=true (grant subawards and subcontracts).
    UsaspendingSubawards,
    /// USAspending spending_by_transaction or award download rows.
    UsaspendingTransactions,
    /// NIH RePORTER v2 projects/search results (screens the title, abstract and terms).
    NihReporter,
}

fn pick(headers: &[String], names: &[&str]) -> Option<String> {
    for n in names {
        if let Some(h) = headers.iter().find(|h| h.eq_ignore_ascii_case(n)) {
            return Some(h.clone());
        }
    }
    None
}

impl Columns {
    pub fn from_preset(p: Preset, headers: &[String]) -> Columns {
        let s = |v: &str| Some(v.to_string());
        match p {
            Preset::UsaspendingSubawards => Columns {
                vendor: s("Sub-Awardee Name"),
                amount: s("Sub-Award Amount"),
                date: s("Sub-Award Date"),
                award: s("Prime Award ID"),
                group: s("Prime Recipient Name"),
                id: s("Sub-Award ID"),
                text: vec!["Sub-Award Description".into()],
            },
            Preset::UsaspendingTransactions => Columns {
                vendor: pick(headers, &["Recipient Name", "recipient_name"]),
                amount: pick(
                    headers,
                    &[
                        "Transaction Amount",
                        "federal_action_obligation",
                        "Award Amount",
                    ],
                ),
                date: pick(headers, &["Action Date", "action_date"]),
                award: pick(headers, &["Award ID", "award_id_piid", "award_id_fain"]),
                group: pick(
                    headers,
                    &["Awarding Sub Agency", "awarding_sub_agency_name"],
                ),
                id: pick(headers, &["internal_id", "generated_internal_id"]),
                text: [
                    "Transaction Description",
                    "transaction_description",
                    "Description",
                    "product_or_service_code_description",
                ]
                .iter()
                .filter_map(|n| pick(headers, &[n]))
                .collect(),
            },
            Preset::NihReporter => Columns {
                vendor: None,
                amount: s("award_amount"),
                date: pick(headers, &["budget_start", "project_start_date"]),
                award: s("project_num"),
                group: s("organization.org_name"),
                id: s("appl_id"),
                text: vec![
                    "project_title".into(),
                    "abstract_text".into(),
                    "pref_terms".into(),
                    "terms".into(),
                ],
            },
            Preset::Auto => Columns {
                vendor: pick(
                    headers,
                    &[
                        "vendor_name",
                        "vendor",
                        "payee",
                        "payee_name",
                        "supplier",
                        "supplier_name",
                        "merchant",
                        "recipient_name",
                        "recipient",
                        "vendor_legal_name",
                        "name1",
                        "contractor_name",
                        "seller_name",
                        "Sub-Awardee Name",
                        "Recipient Name",
                    ],
                ),
                amount: pick(
                    headers,
                    &[
                        "amount",
                        "payment_amount",
                        "check_amount",
                        "amt",
                        "item_net_total",
                        "item_subtotal",
                        "merchandise_amt",
                        "total",
                        "chksubtot",
                        "sum_amount",
                        "Sub-Award Amount",
                        "Transaction Amount",
                        "payment_accounting_net_amount",
                        "value_of_spend",
                    ],
                ),
                date: pick(
                    headers,
                    &[
                        "date",
                        "payment_date",
                        "check_date",
                        "trans_dt",
                        "order_date",
                        "invoice_date",
                        "pymnt_dt",
                        "rundate",
                        "Action Date",
                        "Sub-Award Date",
                    ],
                ),
                award: pick(
                    headers,
                    &[
                        "award",
                        "award_id",
                        "grant",
                        "grant_id",
                        "project",
                        "Prime Award ID",
                        "Award ID",
                    ],
                ),
                group: pick(
                    headers,
                    &[
                        "institution",
                        "agency",
                        "agency_name",
                        "department",
                        "dept_name",
                        "govtunit",
                    ],
                ),
                id: pick(
                    headers,
                    &[
                        "id",
                        "payment_id",
                        "transaction_id",
                        "voucher",
                        "voucher_number",
                    ],
                ),
                text: [
                    "description",
                    "title",
                    "invoice_description",
                    "invoice_desc",
                    "item_description",
                    "brand",
                    "manufacturer",
                    "item_model_number",
                    "product_description",
                    "Transaction Description",
                    "Sub-Award Description",
                    "expense_description",
                    "commodity_description",
                ]
                .iter()
                .filter_map(|n| pick(headers, &[n]))
                .collect(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts() {
        assert_eq!(parse_amount("$1,234.50"), Some(1234.5));
        assert_eq!(parse_amount("(12.00)"), Some(-12.0));
        assert_eq!(parse_amount("-7"), Some(-7.0));
        assert_eq!(parse_amount(""), None);
    }

    #[test]
    fn flattens_nested_json() {
        let v: Value =
            serde_json::json!({"organization": {"org_name": "X U"}, "terms": ["a", "b"]});
        let mut f = Vec::new();
        flatten("", &v, &mut f);
        assert!(f.contains(&("organization.org_name".to_string(), "X U".to_string())));
        assert!(f.contains(&("terms".to_string(), "a; b".to_string())));
    }
}
