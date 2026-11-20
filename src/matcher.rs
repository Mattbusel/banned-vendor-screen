//! Matching a payment line against the loaded lists.
//!
//! Three levels, so a reviewer can tell an answer from a lead:
//! - `confirmed`: the payee is a listed name or alias (legal form aside), a subsidiary the
//!   list names, or the line names a product only a listed company makes.
//! - `likely`: the payee carries a listed company's coined brand ("Hikvision USA", "Novogene
//!   Corporation"), or GLEIF shows a listed company owns 50% or more of it.
//! - `review`: a short acronym another company also uses ("BGI LLC" doing engineering work),
//!   a multi-word name with extra words, or a control-only ownership link.

use crate::families::{compiled_products, CompiledProduct};
use crate::lists::{Entity, Kind, Lists};
use crate::normalize::{
    brand_core, contains_words, core, core_tokens, has_word, is_short_key, legal_core, GENERIC,
    PLACES,
};
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};
use std::sync::LazyLock;

static DBA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:d/b/a|dba|a/k/a|aka|f/k/a|fka|doing business as|formerly)")
        .expect("dba regex")
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Review,
    Likely,
    Confirmed,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Review => "review",
            Level::Likely => "likely",
            Level::Confirmed => "confirmed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum How {
    /// Payee name equals a listed name or alias once legal forms are removed.
    Name,
    /// Payee name equals the listed company's brand ("Hikvision USA Inc" vs "Hangzhou Hikvision ...").
    Brand,
    /// Payee name contains the listed company's coined brand as whole words.
    BrandWords,
    /// The line's description names a product the listed company makes.
    Product,
    /// Payee is a GLEIF affiliate of a listed company.
    Affiliate,
}

#[derive(Debug, Clone, Serialize)]
pub struct Match {
    #[serde(skip)]
    pub entity: usize,
    pub how: How,
    pub level: Level,
    /// The text on the line that matched (payee name or product phrase).
    pub matched_text: String,
    pub note: Option<String>,
    #[serde(skip)]
    pub drone_product: bool,
}

pub struct Index<'a> {
    pub lists: &'a Lists,
    by_key: HashMap<String, Vec<(usize, How)>>,
    /// Multi-word listed names with only legal forms removed ("bgi americas").
    by_full: HashMap<String, Vec<usize>>,
    brand_words: Vec<(Vec<String>, usize)>,
    products: Vec<CompiledProduct>,
}

/// A sector word on the line, matched on word starts (see `normalize::has_word`).
fn sector_word_on_line(e: &Entity, line_lower: &str) -> Option<&'static str> {
    let f = e.family?;
    f.sector
        .words()
        .iter()
        .copied()
        .find(|w| has_word(line_lower, w))
}

/// Phrases that mean the line names a company without buying from it: a contract clause that
/// prohibits it, a lawsuit, an expert witness.
const NOT_A_PURCHASE: &[&str] = &[
    "52.204-23",
    "52.204-25",
    "52.204-26",
    "52.240-1",
    "prohibition on contracting",
    "expert witness",
    "litigation",
    "lawsuit",
    "rip and replace",
    " v. ",
];

impl<'a> Index<'a> {
    pub fn new(lists: &'a Lists) -> Index<'a> {
        let mut by_key: HashMap<String, Vec<(usize, How)>> = HashMap::new();
        let mut by_full: HashMap<String, Vec<usize>> = HashMap::new();
        let mut brand_words = Vec::new();
        for (i, e) in lists.entities.iter().enumerate() {
            let affiliate = matches!(e.kind, Kind::Affiliate { .. });
            for n in &e.names {
                let lc = legal_core(n);
                if !affiliate && lc.contains(' ') {
                    by_full.entry(lc).or_default().push(i);
                }
                let c = core(n);
                if !c.is_empty() {
                    let how = if affiliate { How::Affiliate } else { How::Name };
                    by_key.entry(c.clone()).or_default().push((i, how));
                }
                // Brand keys only for companies with a known family; a brand cut from an
                // arbitrary listed name ("Camel Group" -> "camel") would hit unrelated payees.
                if !affiliate && e.family.is_some() {
                    let b = brand_core(n);
                    if !b.is_empty() && b != c {
                        by_key.entry(b.clone()).or_default().push((i, How::Brand));
                    }
                    let bt: Vec<String> = b.split(' ').map(str::to_string).collect();
                    brand_words.push((bt, i));
                }
            }
        }
        for v in by_key.values_mut() {
            v.sort_by_key(|(i, _)| *i);
            v.dedup_by_key(|(i, _)| *i);
        }
        for v in by_full.values_mut() {
            v.dedup();
        }
        Index {
            lists,
            by_key,
            by_full,
            brand_words,
            products: compiled_products(),
        }
    }

    fn entities_named(&self, maker: &str) -> Vec<usize> {
        let c = core(maker);
        self.by_key
            .get(&c)
            .map(|v| {
                v.iter()
                    .filter(|(_, h)| *h == How::Name)
                    .map(|(i, _)| *i)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Screen one line. `vendor` is the payee; `texts` are description/title/abstract fields.
    pub fn screen(&self, vendor: Option<&str>, texts: &[&str]) -> Vec<Match> {
        let mut out: Vec<Match> = Vec::new();
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        let line_lower = {
            let mut s = vendor.unwrap_or("").to_lowercase();
            for t in texts {
                s.push(' ');
                s.push_str(&t.to_lowercase());
            }
            s
        };

        // Products named on the line (resellers, catalogs, abstracts).
        let mut product_families: BTreeSet<&'static str> = BTreeSet::new();
        for t in texts {
            for p in &self.products {
                if let Some(m) = p.re.find(t) {
                    if product_families.contains(p.family.id) {
                        continue;
                    }
                    if !p.rule.context.is_empty()
                        && !p.rule.context.iter().any(|w| has_word(&line_lower, w))
                    {
                        continue;
                    }
                    let not_purchase = NOT_A_PURCHASE.iter().find(|w| line_lower.contains(*w));
                    let makers = self.entities_named(p.rule.maker);
                    if makers.is_empty() {
                        continue;
                    }
                    product_families.insert(p.family.id);
                    for i in makers {
                        if seen.insert(i) {
                            out.push(Match {
                                entity: i,
                                how: How::Product,
                                level: if not_purchase.is_some() { Level::Review } else { Level::Confirmed },
                                matched_text: m.as_str().to_string(),
                                note: Some(match not_purchase {
                                    Some(w) => format!(
                                        "line names {} but reads as a clause, lawsuit or removal (\"{}\"), not a purchase",
                                        p.rule.label,
                                        w.trim()
                                    ),
                                    None => format!("line names {}", p.rule.label),
                                }),
                                drone_product: p.rule.drone,
                            });
                        }
                    }
                }
            }
        }

        let Some(v) = vendor.filter(|v| !v.trim().is_empty()) else {
            return out;
        };
        // "Acme Inc dba Hikvision Store": screen each name the payee goes by.
        let parts: Vec<&str> = DBA
            .split(v)
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();
        if parts.len() > 1 {
            for p in parts {
                for m in self.screen(Some(p), &[]) {
                    if seen.insert(m.entity) {
                        out.push(m);
                    }
                }
            }
            return out;
        }
        // The listed name itself, legal form aside ("BGI Americas Corp" = "BGI Americas Corporation").
        if let Some(cands) = self.by_full.get(&legal_core(v)) {
            for &i in cands {
                if seen.insert(i) {
                    out.push(Match {
                        entity: i,
                        how: How::Name,
                        level: Level::Confirmed,
                        matched_text: v.to_string(),
                        note: None,
                        drone_product: false,
                    });
                }
            }
        }
        let vtoks = core_tokens(v);
        let vc = vtoks.join(" ");
        let vb = brand_core(v);
        for key in [&vc, &vb] {
            let Some(cands) = self.by_key.get(key.as_str()) else {
                continue;
            };
            for &(i, how) in cands {
                let e = &self.lists.entities[i];
                // A payee's brand is only compared with companies whose brand we know; for
                // the rest, the whole name has to match.
                if key != &vc && e.family.is_none() {
                    continue;
                }
                let (level, note) = match &e.kind {
                    Kind::Affiliate { covered, owner, verdicts } => {
                        if key != &vc {
                            continue;
                        }
                        let lvl = if *covered { Level::Likely } else { Level::Review };
                        (lvl, Some(format!("GLEIF: {owner} controls this payee ({verdicts})")))
                    }
                    Kind::Listed if is_short_key(key) => {
                        let fam = e.family.map(|f| f.id);
                        if fam.is_some_and(|f| product_families.contains(f)) {
                            (Level::Confirmed, Some("short name, confirmed by a product on the same line".into()))
                        } else if let Some(w) = sector_word_on_line(e, &line_lower) {
                            (Level::Confirmed, Some(format!("short name, confirmed by \"{w}\" on the same line")))
                        } else {
                            (
                                Level::Review,
                                Some(format!(
                                    "\"{key}\" alone is a short name unrelated companies also use, and nothing on the line says {}",
                                    e.family.map(|f| format!("{:?}", f.sector).to_lowercase()).unwrap_or_else(|| "what was bought".into())
                                )),
                            )
                        }
                    }
                    Kind::Listed if e.family.is_none() && !key.contains(' ') => (
                        Level::Review,
                        Some(format!("one-word name \"{key}\"; unrelated companies share one-word names, so check the address or ID")),
                    ),
                    Kind::Listed => (Level::Confirmed, None),
                };
                if seen.insert(i) {
                    out.push(Match {
                        entity: i,
                        how,
                        level,
                        matched_text: v.to_string(),
                        note,
                        drone_product: false,
                    });
                } else if let Some(m) = out.iter_mut().find(|m| m.entity == i) {
                    if level > m.level {
                        m.level = level;
                    }
                }
            }
        }
        // Coined brand inside a longer payee name: "Huawei Device USA" -> Huawei.
        for (bt, i) in &self.brand_words {
            if seen.contains(i) || !contains_words(&vtoks, bt) {
                continue;
            }
            let e = &self.lists.entities[*i];
            // "BYD Coach & Bus": a short brand inside a longer name counts only when the
            // line also says what the company makes.
            let short = bt.len() == 1 && is_short_key(&bt[0]);
            let corroborated = if short {
                sector_word_on_line(e, &line_lower)
            } else {
                None
            };
            if short && corroborated.is_none() {
                continue;
            }
            let extra: Vec<&String> = vtoks
                .iter()
                .filter(|w| {
                    !bt.contains(w)
                        && !GENERIC.contains(&w.as_str())
                        && !PLACES.contains(&w.as_str())
                })
                .collect();
            let level = if bt.len() == 1 || extra.is_empty() {
                Level::Likely
            } else {
                Level::Review
            };
            let note = if let Some(w) = corroborated {
                Some(format!(
                    "payee name carries the {} brand, and \"{w}\" on the line says what it sells",
                    e.family.map(|f| f.label).unwrap_or("listed")
                ))
            } else if extra.is_empty() {
                None
            } else {
                Some(format!(
                    "payee name carries the {} brand plus \"{}\"; US subsidiaries and affiliates are rarely on the lists by name",
                    e.family.map(|f| f.label).unwrap_or("listed"),
                    extra.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ")
                ))
            };
            seen.insert(*i);
            out.push(Match {
                entity: *i,
                how: How::BrandWords,
                level,
                matched_text: v.to_string(),
                note,
                drone_product: false,
            });
        }
        out
    }
}
