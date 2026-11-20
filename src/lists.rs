//! Loads the US restricted-party feed (https://github.com/Mattbusel/us-restricted-parties):
//! Senzing JSON Lines for DoD 1260H, Section 889, the FCC Covered List, UFLPA and BIOSECURE,
//! plus the GLEIF-derived affiliates file for ownership.
//!
//! The loader fails closed. A missing list, a list with no names, or a feed older than the
//! allowed age is an error, not a warning: a screen that ran against nothing is not a pass.

use crate::dates;
use crate::families::{family_of, Family};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const REQUIRED: &[(&str, &str)] = &[
    (
        "US_DOD_1260H",
        "DoD Chinese Military Companies (Section 1260H)",
    ),
    (
        "US_FAR_889",
        "Section 889 covered telecom and video surveillance",
    ),
    ("US_FCC_COVERED", "FCC Covered List"),
    ("US_DHS_UFLPA", "UFLPA Entity List"),
    (
        "US_BIOSECURE",
        "BIOSECURE biotechnology companies of concern (interim)",
    ),
];

pub const FEED_URL: &str =
    "https://github.com/Mattbusel/us-restricted-parties/releases/latest/download/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Listed,
    /// GLEIF-derived affiliate. `covered` is true when the derived verdict is "definitely"
    /// (50% or more owned); control-only links are evidence to review, not a rule hit.
    Affiliate {
        covered: bool,
        owner: String,
        verdicts: String,
    },
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub record_id: String,
    pub source: String,
    pub list_name: String,
    pub name: String,
    pub names: Vec<String>,
    pub listed_on: Option<String>,
    pub parent: Option<String>,
    pub source_url: Option<String>,
    pub kind: Kind,
    pub family: Option<&'static Family>,
}

#[derive(Debug)]
pub struct Lists {
    pub entities: Vec<Entity>,
    pub dir: PathBuf,
    pub built_at: Option<String>,
    pub counts: BTreeMap<String, usize>,
    pub affiliates: usize,
}

#[derive(Debug)]
pub enum ListError {
    Missing(String),
    Empty(String),
    Stale {
        built_at: String,
        age_days: i64,
        max: i64,
    },
    NoManifest,
    Io(String),
}

impl std::fmt::Display for ListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListError::Missing(s) => write!(
                f,
                "list {s} is not in the lists directory. A screen without it cannot pass. Download: {FEED_URL}all.jsonl"
            ),
            ListError::Empty(s) => write!(
                f,
                "list {s} loaded with no names. A list that loads empty matches nothing, so this run stops here."
            ),
            ListError::Stale { built_at, age_days, max } => write!(
                f,
                "the lists were built {built_at}, {age_days} days ago (limit {max}). The lists move between reviews; fetch a current copy from {FEED_URL} or raise --max-age-days on purpose."
            ),
            ListError::NoManifest => write!(
                f,
                "manifest.json is missing, so the age of the lists cannot be shown. Download {FEED_URL}manifest.json or pass --max-age-days 0 to skip the check on purpose."
            ),
            ListError::Io(s) => write!(f, "{s}"),
        }
    }
}

fn str_field(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_string)
}

fn names_of(v: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(n) = str_field(v, "NAME_ORG") {
        out.push(n);
    }
    if let Some(arr) = v.get("NAMES").and_then(Value::as_array) {
        for n in arr {
            if let Some(s) = n.get("NAME_ORG").and_then(Value::as_str) {
                if !out.iter().any(|o| o == s) {
                    out.push(s.to_string());
                }
            }
        }
    }
    out.retain(|s| !s.trim().is_empty());
    out
}

fn read_jsonl(path: &Path) -> Result<Vec<Value>, ListError> {
    let text =
        fs::read_to_string(path).map_err(|e| ListError::Io(format!("{}: {e}", path.display())))?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line)
            .map_err(|e| ListError::Io(format!("{} line {}: {e}", path.display(), i + 1)))?;
        out.push(v);
    }
    Ok(out)
}

fn listed(v: &Value) -> Entity {
    let names = names_of(v);
    Entity {
        record_id: str_field(v, "RECORD_ID").unwrap_or_default(),
        source: str_field(v, "DATA_SOURCE").unwrap_or_default(),
        list_name: str_field(v, "LIST_NAME").unwrap_or_default(),
        name: names.first().cloned().unwrap_or_default(),
        family: family_of(&names),
        names,
        listed_on: str_field(v, "LISTED_ON").and_then(|d| dates::parse_date(&d)),
        parent: str_field(v, "PARENT_NAME"),
        source_url: str_field(v, "SOURCE_URL"),
        kind: Kind::Listed,
    }
}

/// Affiliates whose listed owner is on one of the five lists this tool screens.
fn affiliate(v: &Value) -> Option<Entity> {
    let proof = v.get("OWNERSHIP_PROOF")?.as_object()?;
    let mut owner = None;
    let mut sources: Vec<String> = Vec::new();
    for (_rule, p) in proof {
        for step in p
            .get("proof")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if step.get("round").and_then(Value::as_i64) == Some(0) {
                for l in step
                    .get("lists")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if let Some(l) = l.as_str() {
                        if REQUIRED.iter().any(|(id, _)| *id == l)
                            && !sources.iter().any(|s| s == l)
                        {
                            sources.push(l.to_string());
                            owner = owner.or_else(|| {
                                step.get("name").and_then(Value::as_str).map(str::to_string)
                            });
                        }
                    }
                }
            }
        }
    }
    let owner = owner?;
    let verdicts = v
        .get("OWNERSHIP_VERDICTS")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .map(|(k, v)| format!("{k}: {}", v.as_str().unwrap_or("?")))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let covered = v
        .get("OWNERSHIP_VERDICTS")
        .and_then(Value::as_object)
        .map(|m| m.values().any(|x| x.as_str() == Some("definitely")))
        .unwrap_or(false);
    let names = names_of(v);
    Some(Entity {
        record_id: str_field(v, "RECORD_ID").unwrap_or_default(),
        source: sources.join("+"),
        list_name:
            "Affiliate of a listed company (GLEIF ownership links; evidence, not determination)"
                .into(),
        name: names.first().cloned().unwrap_or_default(),
        family: family_of(std::slice::from_ref(&owner)),
        names,
        listed_on: None,
        parent: Some(owner.clone()),
        source_url: str_field(v, "LEI_NUMBER")
            .map(|l| format!("https://search.gleif.org/#/record/{l}")),
        kind: Kind::Affiliate {
            covered,
            owner,
            verdicts,
        },
    })
}

impl Lists {
    /// Load every `*.jsonl` list in `dir` (or `all.jsonl`), the affiliates file, and the manifest.
    pub fn load(dir: &Path, max_age_days: i64, with_affiliates: bool) -> Result<Lists, ListError> {
        let mut values = Vec::new();
        let all = dir.join("all.jsonl");
        if all.exists() {
            values.extend(read_jsonl(&all)?);
        } else {
            for (id, _) in REQUIRED {
                let p = dir.join(format!("{}.jsonl", id.to_lowercase()));
                if p.exists() {
                    values.extend(read_jsonl(&p)?);
                }
            }
        }
        let mut entities: Vec<Entity> = values
            .iter()
            .filter(|v| str_field(v, "DATA_SOURCE").is_some())
            .map(listed)
            .collect();
        let mut counts = BTreeMap::new();
        for (id, _) in REQUIRED {
            let n = entities
                .iter()
                .filter(|e| e.source == *id)
                .map(|e| e.names.len())
                .sum::<usize>();
            let present = entities.iter().any(|e| e.source == *id);
            if !present {
                return Err(ListError::Missing(id.to_string()));
            }
            if n == 0 {
                return Err(ListError::Empty(id.to_string()));
            }
            counts.insert(
                id.to_string(),
                entities.iter().filter(|e| e.source == *id).count(),
            );
        }
        let mut affiliates = 0;
        if with_affiliates {
            let p = dir.join("affiliates.senzing.jsonl");
            if !p.exists() {
                return Err(ListError::Missing(
                    "affiliates.senzing.jsonl (ownership links; pass --no-affiliates to screen names only, on purpose)"
                        .into(),
                ));
            }
            for v in read_jsonl(&p)? {
                if let Some(e) = affiliate(&v) {
                    affiliates += 1;
                    entities.push(e);
                }
            }
        }
        let manifest = dir.join("manifest.json");
        let built_at = fs::read_to_string(&manifest)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| str_field(&v, "built_at"));
        if max_age_days > 0 {
            let b = built_at.clone().ok_or(ListError::NoManifest)?;
            let age = dates::today_days() - dates::to_days(&b).ok_or(ListError::NoManifest)?;
            if age > max_age_days {
                return Err(ListError::Stale {
                    built_at: b,
                    age_days: age,
                    max: max_age_days,
                });
            }
        }
        Ok(Lists {
            entities,
            dir: dir.to_path_buf(),
            built_at,
            counts,
            affiliates,
        })
    }
}
