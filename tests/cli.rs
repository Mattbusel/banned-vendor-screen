use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_banned-vendor-screen");

fn today() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        / 86_400;
    // civil_from_days
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

fn rec(source: &str, id: &str, names: &[&str], listed: &str, parent: Option<&str>) -> String {
    let ns: Vec<String> = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            format!(
                r#"{{"NAME_TYPE": "{}", "NAME_ORG": "{}"}}"#,
                if i == 0 { "PRIMARY" } else { "ALIAS" },
                n
            )
        })
        .collect();
    format!(
        r#"{{"DATA_SOURCE": "{source}", "RECORD_ID": "{id}", "NAME_ORG": "{}", "NAMES": [{}], "LIST_NAME": "{source} list", "LISTED_ON": "{listed}", "SOURCE_URL": "https://example.gov/{id}"{}}}"#,
        names[0],
        ns.join(", "),
        parent
            .map(|p| format!(r#", "PARENT_NAME": "{p}""#))
            .unwrap_or_default()
    )
}

fn affiliate(id: &str, name: &str, owner: &str, verdict: &str) -> String {
    format!(
        r#"{{"DATA_SOURCE": "US_RESTRICTED_AFFILIATES", "RECORD_ID": "{id}", "NAME_ORG": "{name}", "NAMES": [{{"NAME_TYPE": "PRIMARY", "NAME_ORG": "{name}"}}], "LEI_NUMBER": "{id}", "OWNERSHIP_VERDICTS": {{"any-list-50": "{verdict}"}}, "OWNERSHIP_PROOF": {{"any-list-50": {{"verdict": "{verdict}", "proof": [{{"entity": "x", "verdict": "listed", "round": 0, "lists": ["US_DOD_1260H"], "name": "{owner}"}}]}}}}}}"#
    )
}

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str, built_at: &str, drop: Option<&str>, empty: Option<&str>) -> Fixture {
        let dir = std::env::temp_dir().join(format!("bvs-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut lines = vec![
            rec(
                "US_DOD_1260H",
                "bgi-group",
                &["BGI Group"],
                "2026-06-10",
                None,
            ),
            rec(
                "US_DOD_1260H",
                "bgi-americas",
                &["BGI Americas Corporation"],
                "2026-06-10",
                Some("BGI Group"),
            ),
            rec(
                "US_DOD_1260H",
                "bgi-genomics",
                &["BGI Genomics Co., Ltd.", "BGI"],
                "2026-06-10",
                Some("BGI Group"),
            ),
            rec(
                "US_DOD_1260H",
                "mgi",
                &["MGI Tech Co., Ltd.", "MGI"],
                "2026-06-10",
                None,
            ),
            rec(
                "US_DOD_1260H",
                "dji",
                &["Shenzhen DJI Technology Co., Ltd.", "DJI"],
                "2026-06-10",
                None,
            ),
            rec(
                "US_DOD_1260H",
                "novogene",
                &["Novogene Company Limited", "Novogene"],
                "2026-06-10",
                None,
            ),
            rec(
                "US_DOD_1260H",
                "tencent",
                &["Tencent Holdings Limited", "Tencent"],
                "2026-06-10",
                None,
            ),
            rec(
                "US_DOD_1260H",
                "hik",
                &[
                    "Hangzhou Hikvision Digital Technology Co., Ltd.",
                    "Hikvision",
                ],
                "2026-06-10",
                None,
            ),
            rec(
                "US_FAR_889",
                "889-hik",
                &["Hangzhou Hikvision Digital Technology Company"],
                "2019-08-13",
                None,
            ),
            rec(
                "US_FAR_889",
                "889-zte",
                &["ZTE Corporation"],
                "2019-08-13",
                None,
            ),
            rec(
                "US_FCC_COVERED",
                "fcc-zte",
                &["ZTE Corporation"],
                "2021-03-12",
                None,
            ),
            rec(
                "US_FCC_COVERED",
                "fcc-kaspersky",
                &["Kaspersky Lab, Inc"],
                "2024-07-23",
                None,
            ),
            rec(
                "US_DHS_UFLPA",
                "uflpa-bgi",
                &["BGI Genomics Co., Ltd.", "BGI Genomics"],
                "2026-10-29",
                None,
            ),
            rec(
                "US_DHS_UFLPA",
                "uflpa-camel",
                &["Camel Group Co., Ltd."],
                "2024-01-01",
                None,
            ),
            rec(
                "US_BIOSECURE",
                "bio-bgi",
                &["BGI Americas Corporation"],
                "2026-06-10",
                Some("BGI Group"),
            ),
        ];
        if let Some(d) = drop {
            lines.retain(|l| !l.contains(&format!("\"DATA_SOURCE\": \"{d}\"")));
        }
        if let Some(e) = empty {
            lines.retain(|l| !l.contains(&format!("\"DATA_SOURCE\": \"{e}\"")));
            lines.push(format!(
                r#"{{"DATA_SOURCE": "{e}", "RECORD_ID": "x", "NAMES": []}}"#
            ));
        }
        std::fs::write(dir.join("all.jsonl"), lines.join("\n")).unwrap();
        std::fs::write(
            dir.join("affiliates.senzing.jsonl"),
            [
                affiliate(
                    "lei-riot",
                    "RIOT GAMES, INC.",
                    "Tencent Holdings Limited",
                    "control_only",
                ),
                affiliate(
                    "lei-super",
                    "Supercell Oy",
                    "Tencent Holdings Limited",
                    "definitely",
                ),
            ]
            .join("\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            format!(r#"{{"built_at": "{built_at}T00:00:00Z"}}"#),
        )
        .unwrap();
        Fixture { dir }
    }

    fn csv(&self, name: &str, body: &str) -> PathBuf {
        let p = self.dir.join(name);
        std::fs::write(&p, body).unwrap();
        p
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn run(lists: &Path, args: &[&str]) -> (i32, serde_json::Value, String) {
    let json = lists.join(format!("report-{}.json", args.len()));
    let out = Command::new(BIN)
        .arg("--lists")
        .arg(lists)
        .arg("--json")
        .arg(&json)
        .arg("--markdown")
        .arg(lists.join("summary.md"))
        .args(args)
        .env_remove("GITHUB_STEP_SUMMARY")
        .output()
        .unwrap();
    let code = out.status.code().unwrap();
    let report = std::fs::read_to_string(&json)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(serde_json::Value::Null);
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    (code, report, text)
}

const PAYMENTS: &str = "vendor_name,amount,payment_date,description,department\n\
Novogene Corporation Inc,4828.00,2026-07-01,Whole genome sequencing service,Genetics\n\
BGI Americas Corp,1200.50,2026-08-02,library prep,Genetics\n\
B&H Foto & Electronics Corp,15999.00,2026-01-15,DJI MATRICE 350 RTK COMBO,Forestry\n\
Hikvision USA Inc,880.00,2025-03-01,DS-2CD2143G2-I dome camera,Facilities\n\
BGI LLC,500000.00,2026-05-01,Engineering support services,Aviation\n\
Employment Agency of Djibouti,1000.00,2026-02-01,Temporary labor,Overseas\n\
Aztec Door & Hardware LLC,2000.00,2026-02-01,Doors and frames,Facilities\n\
Camel Inc,300.00,2026-02-01,Office chairs,Facilities\n\
RIOT GAMES INC,99.00,2026-02-01,Esports club fee,Athletics\n\
Supercell Oy,49.00,2026-02-01,In-app purchase,Athletics\n\
Fisher Scientific,320.00,2026-02-01,Pipette tips,Genetics\n\
TSP Tankers Holdings LLC,3870000.00,2024-01-26,TANKER VOYAGE CHARTER MAL-UAE-DJI,Navy\n\
Enterprise Engineering Inc,1030343.00,2024-09-09,PIPELINE UPDATES AT DJI AND SBY,Navy\n\
Benco Dental Supply Co,2317.00,2025-07-29,MATRICE DNTL GRAY 100S,Dental\n\
PremierSDN LLC,15586.00,2024-09-12,INCORPORATE 52.204-23 PROHIBITION ON CONTRACTING FOR KASPERSKY,USCG\n";

/// Lines from real USAspending descriptions that a substring screen gets wrong.
#[test]
fn airport_codes_dental_bands_and_clauses_are_not_purchases() {
    let f = Fixture::new("noise", &today(), None, None);
    let p = f.csv("payments.csv", PAYMENTS);
    let (_, r, text) = run(&f.dir, &[p.to_str().unwrap()]);
    // "DJI" as Djibouti's airport code, "Matrice" as a dental matrix band: no hit.
    assert_eq!(level_of(&r, "TSP Tankers Holdings LLC"), None, "{text}");
    assert_eq!(level_of(&r, "Enterprise Engineering Inc"), None, "{text}");
    assert_eq!(level_of(&r, "Benco Dental Supply Co"), None, "{text}");
    // A clause that bans Kaspersky is not a purchase from Kaspersky.
    assert_eq!(level_of(&r, "PremierSDN LLC").as_deref(), Some("review"));
}

fn level_of(report: &serde_json::Value, vendor: &str) -> Option<String> {
    report["hits"]
        .as_array()?
        .iter()
        .find(|h| h["vendor"].as_str() == Some(vendor))
        .map(|h| h["level"].as_str().unwrap().to_string())
}

#[test]
fn screens_owners_subsidiaries_and_products() {
    let f = Fixture::new("main", &today(), None, None);
    let p = f.csv("payments.csv", PAYMENTS);
    let (code, r, text) = run(&f.dir, &["--funds", "federal-grant", p.to_str().unwrap()]);
    assert_eq!(code, 1, "{text}");
    assert_eq!(
        level_of(&r, "Novogene Corporation Inc").as_deref(),
        Some("confirmed")
    );
    assert_eq!(
        level_of(&r, "BGI Americas Corp").as_deref(),
        Some("confirmed")
    );
    assert_eq!(
        level_of(&r, "B&H Foto & Electronics Corp").as_deref(),
        Some("confirmed")
    );
    assert_eq!(
        level_of(&r, "Hikvision USA Inc").as_deref(),
        Some("confirmed")
    );
    // A short acronym on an unrelated engineering line is a lead, not a hit.
    assert_eq!(level_of(&r, "BGI LLC").as_deref(), Some("review"));
    // Whole words only, and no brand cut from arbitrary listed names.
    assert_eq!(level_of(&r, "Employment Agency of Djibouti"), None);
    assert_eq!(level_of(&r, "Aztec Door & Hardware LLC"), None);
    assert_eq!(level_of(&r, "Camel Inc"), None);
    assert_eq!(level_of(&r, "Fisher Scientific"), None);
    // Ownership: control-only is review, 50%+ owned is likely.
    assert_eq!(level_of(&r, "RIOT GAMES INC").as_deref(), Some("review"));
    assert_eq!(level_of(&r, "Supercell Oy").as_deref(), Some("likely"));

    let hits = r["hits"].as_array().unwrap();
    let bgi = hits
        .iter()
        .find(|h| h["vendor"] == "BGI Americas Corp")
        .unwrap();
    let chain = bgi["chain"].to_string();
    assert!(chain.contains("subsidiary of BGI Group"), "{chain}");
    let dji = hits
        .iter()
        .find(|h| h["vendor"] == "B&H Foto & Electronics Corp")
        .unwrap();
    let asda = dji["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["rule"] == "ASDA-1825")
        .unwrap();
    assert_eq!(asda["status"], "in_force_at_line_date");
    let hik = hits
        .iter()
        .find(|h| h["vendor"] == "Hikvision USA Inc")
        .unwrap();
    let g = hik["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["rule"] == "889-GRANTS")
        .unwrap();
    assert_eq!(g["status"], "in_force_at_line_date");
    // Failing total = confirmed + likely lines.
    assert_eq!(r["totals"]["failing_rows"], 5);
    let usd = r["totals"]["failing_usd"].as_f64().unwrap();
    assert!(
        (usd - (4828.0 + 1200.5 + 15999.0 + 880.0 + 49.0)).abs() < 0.01,
        "{usd}"
    );
    assert_eq!(r["verdict"], "fail");
}

#[test]
fn fail_on_and_since_control_the_exit_code() {
    let f = Fixture::new("since", &today(), None, None);
    let p = f.csv("payments.csv", PAYMENTS);
    let (code, _, _) = run(&f.dir, &["--fail-on", "never", p.to_str().unwrap()]);
    assert_eq!(code, 0);
    let (code, r, _) = run(&f.dir, &["--since", "2026-09-01", p.to_str().unwrap()]);
    assert_eq!(code, 0, "{r}");
    let clean = f.csv(
        "clean.csv",
        "vendor_name,amount\nFisher Scientific,10\nAztec Door,5\n",
    );
    let (code, r, _) = run(&f.dir, &[clean.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert_eq!(r["verdict"], "pass");
}

#[test]
fn refuses_missing_empty_or_stale_lists() {
    let f = Fixture::new("missing", &today(), Some("US_DHS_UFLPA"), None);
    let p = f.csv("p.csv", PAYMENTS);
    let (code, _, text) = run(&f.dir, &[p.to_str().unwrap()]);
    assert_eq!(code, 3);
    assert!(text.contains("US_DHS_UFLPA"), "{text}");

    let f = Fixture::new("empty", &today(), None, Some("US_FCC_COVERED"));
    let p = f.csv("p.csv", PAYMENTS);
    let (code, _, text) = run(&f.dir, &[p.to_str().unwrap()]);
    assert_eq!(code, 3);
    assert!(text.contains("no names"), "{text}");

    let f = Fixture::new("stale", "2020-01-01", None, None);
    let p = f.csv("p.csv", PAYMENTS);
    let (code, _, text) = run(&f.dir, &[p.to_str().unwrap()]);
    assert_eq!(code, 3);
    assert!(text.contains("days ago"), "{text}");
    let (code, _, _) = run(&f.dir, &["--max-age-days", "0", p.to_str().unwrap()]);
    assert_eq!(code, 1);
}

#[test]
fn award_text_mode_finds_planned_sequencer_use() {
    let f = Fixture::new("awards", &today(), None, None);
    let p = f.dir.join("awards.json");
    std::fs::write(
        &p,
        r#"{"results": [
          {"appl_id": 1, "project_num": "5R01HG000001-03", "award_amount": 512000, "project_start_date": "2025-07-01T00:00:00",
           "organization": {"org_name": "EXAMPLE UNIVERSITY"}, "project_title": "Population genomics",
           "abstract_text": "Libraries will be sequenced on the DNBSEQ-G400 platform."},
          {"appl_id": 2, "project_num": "5R01HG000002-01", "award_amount": 300000, "project_start_date": "2025-07-01T00:00:00",
           "organization": {"org_name": "OTHER UNIVERSITY"}, "project_title": "Mouse models", "abstract_text": "No vendor named."}
        ]}"#,
    )
    .unwrap();
    let (code, r, text) = run(&f.dir, &["--preset", "nih-reporter", p.to_str().unwrap()]);
    assert_eq!(code, 1, "{text}");
    let hits = r["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["group"], "EXAMPLE UNIVERSITY");
    assert_eq!(hits[0]["award"], "5R01HG000001-03");
    assert_eq!(hits[0]["amount"], 512000.0);
}

#[test]
fn unknown_column_is_an_input_error() {
    let f = Fixture::new("cols", &today(), None, None);
    let p = f.csv("p.csv", PAYMENTS);
    let (code, _, text) = run(&f.dir, &["--vendor", "payee", p.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(text.contains("not in the file"), "{text}");
}
