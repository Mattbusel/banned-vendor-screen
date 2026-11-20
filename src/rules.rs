//! What each hit means, under which rule, as of the line's own date.
//!
//! Dates are the rules' own: 2 CFR 200.216 (Section 889 for grants) since Aug 13 2020; FAR
//! 52.204-25 since Aug 13 2019; American Security Drone Act sec. 1823 (agency purchases) since
//! Dec 22 2023 and sec. 1825 (federal grant and contract funds) since Dec 22 2025; FY2024 NDAA
//! sec. 805 (DoD contracts with 1260H entities) direct from Jun 30 2026 and through suppliers
//! from Jun 30 2027; UFLPA import presumption from each entity's listing date.

use crate::lists::Entity;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Funds {
    /// Federal grant or cooperative agreement money (research awards, subawards).
    FederalGrant,
    /// Federal contract money.
    FederalContract,
    /// Department of Defense contract money.
    Dod,
    /// A federal agency's own purchase.
    Agency,
    /// State, local or institutional money.
    NonFederal,
    /// Not known: every rule is shown with the funds it applies to.
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleStatus {
    pub rule: &'static str,
    pub name: &'static str,
    pub citation: &'static str,
    pub effective: Option<String>,
    /// in_force_at_line_date | before_effective_date | effective_date_pending | informational | line_date_unknown
    pub status: &'static str,
    pub applies_to: &'static str,
    /// False when the funds given with --funds are not money this rule reaches.
    pub applies_to_these_funds: bool,
}

struct Rule {
    id: &'static str,
    name: &'static str,
    citation: &'static str,
    effective: Option<&'static str>,
    applies_to: &'static str,
    funds: &'static [Funds],
}

const R889_GRANTS: Rule = Rule {
    id: "889-GRANTS",
    name: "Grant and loan funds may not buy covered telecom or video surveillance equipment or services from Huawei, ZTE, Hytera, Hikvision, Dahua or their subsidiaries and affiliates",
    citation: "2 CFR 200.216; Pub. L. 115-232 sec. 889",
    effective: Some("2020-08-13"),
    applies_to: "federal grants, cooperative agreements and subawards",
    funds: &[Funds::FederalGrant],
};
const R889_CONTRACTS: Rule = Rule {
    id: "889-CONTRACTS",
    name: "Agencies may not buy, or contract with entities that use, covered telecom or video surveillance equipment or services",
    citation: "FAR 52.204-25",
    effective: Some("2019-08-13"),
    applies_to: "federal contracts and agency purchases",
    funds: &[Funds::FederalContract, Funds::Dod, Funds::Agency],
};
const ASDA_1823: Rule = Rule {
    id: "ASDA-1823",
    name: "Agencies may not buy drones made by covered foreign entities (exemptions exist for testing, training and counter-UAS work)",
    citation: "Pub. L. 118-31 sec. 1823; FAR 52.240-1",
    effective: Some("2023-12-22"),
    applies_to: "federal agency purchases",
    funds: &[Funds::Agency, Funds::FederalContract, Funds::Dod],
};
const ASDA_1825: Rule = Rule {
    id: "ASDA-1825",
    name: "Federal grant and contract funds may not buy or operate drones made by covered foreign entities",
    citation: "Pub. L. 118-31 sec. 1825",
    effective: Some("2025-12-22"),
    applies_to: "federal grants, cooperative agreements and contracts",
    funds: &[Funds::FederalGrant, Funds::FederalContract, Funds::Dod],
};
const NDAA_805: Rule = Rule {
    id: "NDAA24-805",
    name: "DoD may not contract with 1260H entities: directly from Jun 30 2026, through suppliers from Jun 30 2027",
    citation: "Pub. L. 118-31 sec. 805",
    effective: Some("2026-06-30"),
    applies_to: "Department of Defense contracts and their supply chains",
    funds: &[Funds::Dod],
};
const BIOSECURE: Rule = Rule {
    id: "BIOSECURE-851",
    name: "No federal contracts, grants or loans for biotechnology equipment or services from a biotechnology company of concern (OMB list due Dec 18 2026; BGI and MGI were listed before Dec 18 2025, so existing contracts get no grandfathering)",
    citation: "FY2026 NDAA sec. 851",
    effective: None,
    applies_to: "federal contracts, grants and loans",
    funds: &[Funds::FederalGrant, Funds::FederalContract, Funds::Dod, Funds::Agency],
};
const UFLPA: Rule = Rule {
    id: "UFLPA-IMPORT",
    name: "Goods made by a listed entity are presumed made with forced labor and can be detained at import",
    citation: "Pub. L. 117-78 sec. 3",
    effective: None,
    applies_to: "any import, whoever pays",
    funds: &[
        Funds::FederalGrant,
        Funds::FederalContract,
        Funds::Dod,
        Funds::Agency,
        Funds::NonFederal,
    ],
};
const FCC: Rule = Rule {
    id: "FCC-COVERED",
    name: "No new FCC equipment authorizations; FCC-administered subsidies (USF, E-Rate) may not buy or maintain the equipment",
    citation: "47 CFR 1.50002; Secure and Trusted Communications Networks Act",
    effective: None,
    applies_to: "FCC subsidies; information only for research funds",
    funds: &[],
};

fn status(
    rule: &Rule,
    effective: Option<&str>,
    line_date: Option<&str>,
    funds: Funds,
) -> RuleStatus {
    let st = if rule.funds.is_empty() {
        "informational"
    } else {
        match (effective, line_date) {
            (None, _) => "effective_date_pending",
            (Some(_), None) => "line_date_unknown",
            (Some(e), Some(d)) if d >= e => "in_force_at_line_date",
            _ => "before_effective_date",
        }
    };
    RuleStatus {
        rule: rule.id,
        name: rule.name,
        citation: rule.citation,
        effective: effective.map(str::to_string),
        status: st,
        applies_to: rule.applies_to,
        applies_to_these_funds: funds == Funds::Unknown || rule.funds.contains(&funds),
    }
}

/// Rules reached by the matched entities, as of the line date. `drone` is `Some(true)` when the
/// line names a drone product, `Some(false)` for another product, `None` when only the payee matched.
pub fn rules_for(
    entities: &[&Entity],
    drone: Option<bool>,
    line_date: Option<&str>,
    funds: Funds,
) -> Vec<RuleStatus> {
    let mut out: Vec<RuleStatus> = Vec::new();
    let mut push = |r: RuleStatus| {
        if !out.iter().any(|o| o.rule == r.rule) {
            out.push(r);
        }
    };
    let has = |src: &str| {
        entities
            .iter()
            .any(|e| e.source.split('+').any(|s| s == src))
    };
    let fam = |id: &str| {
        entities
            .iter()
            .any(|e| e.family.is_some_and(|f| f.id == id))
    };
    let sec889 = has("US_FAR_889") || entities.iter().any(|e| e.family.is_some_and(|f| f.sec889));
    if sec889 {
        push(status(
            &R889_GRANTS,
            R889_GRANTS.effective,
            line_date,
            funds,
        ));
        push(status(
            &R889_CONTRACTS,
            R889_CONTRACTS.effective,
            line_date,
            funds,
        ));
    }
    let drone_maker = fam("dji")
        || entities
            .iter()
            .any(|e| e.name.to_lowercase().contains("autel robotics"));
    if drone_maker && drone != Some(false) {
        push(status(&ASDA_1823, ASDA_1823.effective, line_date, funds));
        push(status(&ASDA_1825, ASDA_1825.effective, line_date, funds));
    }
    if has("US_BIOSECURE") {
        push(status(&BIOSECURE, None, line_date, funds));
    }
    if has("US_DHS_UFLPA") {
        let listed = entities
            .iter()
            .filter(|e| e.source.split('+').any(|s| s == "US_DHS_UFLPA"))
            .filter_map(|e| e.listed_on.as_deref())
            .min();
        push(status(&UFLPA, listed, line_date, funds));
    }
    if has("US_DOD_1260H") {
        push(status(&NDAA_805, NDAA_805.effective, line_date, funds));
    }
    if has("US_FCC_COVERED") {
        push(status(&FCC, None, line_date, funds));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lists::Kind;

    fn ent(source: &str, name: &str, listed: &str) -> Entity {
        let names = vec![name.to_string()];
        Entity {
            record_id: name.into(),
            source: source.into(),
            list_name: String::new(),
            name: name.into(),
            family: crate::families::family_of(&names),
            names,
            listed_on: Some(listed.into()),
            parent: None,
            source_url: None,
            kind: Kind::Listed,
        }
    }

    #[test]
    fn hikvision_under_889_by_date() {
        let e = ent(
            "US_FAR_889",
            "Hangzhou Hikvision Digital Technology Company",
            "2019-08-13",
        );
        let r = rules_for(&[&e], None, Some("2021-03-01"), Funds::FederalGrant);
        let g = r.iter().find(|r| r.rule == "889-GRANTS").unwrap();
        assert_eq!(g.status, "in_force_at_line_date");
        assert!(g.applies_to_these_funds);
        let r = rules_for(&[&e], None, Some("2020-01-01"), Funds::FederalGrant);
        assert_eq!(
            r.iter().find(|r| r.rule == "889-GRANTS").unwrap().status,
            "before_effective_date"
        );
    }

    #[test]
    fn dji_drone_rules() {
        let e = ent(
            "US_DOD_1260H",
            "Shenzhen DJI Technology Co., Ltd.",
            "2026-06-10",
        );
        let r = rules_for(&[&e], Some(true), Some("2026-01-15"), Funds::FederalGrant);
        assert_eq!(
            r.iter().find(|r| r.rule == "ASDA-1825").unwrap().status,
            "in_force_at_line_date"
        );
        let a = r.iter().find(|r| r.rule == "ASDA-1823").unwrap();
        assert!(!a.applies_to_these_funds);
        assert_eq!(
            r.iter().find(|r| r.rule == "NDAA24-805").unwrap().status,
            "before_effective_date"
        );
    }
}
