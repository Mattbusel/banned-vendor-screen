# banned-vendor-screen: federal research money already pays vendors the US has banned

[![ci](https://github.com/Mattbusel/banned-vendor-screen/actions/workflows/ci.yml/badge.svg)](https://github.com/Mattbusel/banned-vendor-screen/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/banned-vendor-screen.svg)](https://crates.io/crates/banned-vendor-screen)

Federal research money already pays vendors the government has banned, and almost nobody is counting it. We pointed this tool at public federal data on Nov 20 2026:

- A **NIST research grant** to Virginia Commonwealth University (60NANB26D232, $1,000,000, Sep 9 2026) lists a **Unitree H1-2 humanoid robot** among the equipment it buys. DoD put Unitree's maker on the Section 1260H list 91 days earlier.
- **NIH awards** at UCLA, MD Anderson, Michigan, Stanford and Harvard Medical School name **BGI/MGI** platforms (DNBSEQ, Stereo-seq, STOmics) in their FY2025-26 abstracts: 10 award-years, $4,249,852. All three are BGI Group or MGI products; MGI Tech is on DoD 1260H, UFLPA and the interim BIOSECURE list.
- **EPA grants** of $22,975,000 to two school bus fleets name **17 Autel DC fast chargers**. Autel's maker is on the 1260H list.
- **Federal agencies bought DJI drones** in 21 transactions netting $863,535 after the American Security Drone Act banned agency drone purchases from covered makers (Dec 22 2023; exemptions exist for testing, training and counter-UAS work). The payee on most of them is a reseller, so a name screen of the payee passes.
- An **Army Research Office** contractor paid **Novogene** $168,315 for DNA extraction under W911NF22C0064. Novogene is on DoD 1260H and the interim BIOSECURE list, and from Jun 30 2027 FY2024 NDAA sec. 805 reaches DoD contractors' suppliers.

None of these show up in a screen that checks payee names against OFAC and the Consolidated Screening List. Four of the five lists that matter here are in neither, the payee is often a reseller, and the subsidiaries are rarely on any list by name.

The same day, the keyword searches that most people would use returned 1,869 NIH projects (1,859 of them for "MGI" alone) and 724 USAspending subawards for listed brands such as "BGI", "DJI" and "ZTE". Almost all of them are Bemidji school districts, Djibouti airport contracts, Aztec doors, dental matrix bands and a defense engineering firm called BGI LLC. Substring matching is not compliance. This tool confirmed 10 and 1 of them, and puts "BGI LLC" in a review pile instead of failing your build on it.

`banned-vendor-screen` screens payment lines, subawards or award text against **DoD 1260H, Section 889, the FCC Covered List, UFLPA and BIOSECURE**, including **named subsidiaries, GLEIF owners and product names**, and **exits 1 when a covered vendor is found**. It refuses to run on lists that are missing, empty or stale.

Not legal advice. A hit is evidence for a reviewer, not a determination; a paper or abstract naming a product does not prove the award paid for it.

## Quick start

```sh
cargo install banned-vendor-screen
mkdir lists && for f in all.jsonl affiliates.senzing.jsonl manifest.json; do
  curl -fsSL -o lists/$f https://github.com/Mattbusel/us-restricted-parties/releases/latest/download/$f
done
banned-vendor-screen --lists lists --funds federal-grant payments.csv
```

Prebuilt binaries for Linux and Windows are on the [releases page](https://github.com/Mattbusel/banned-vendor-screen/releases); on macOS, `cargo install` (the action does this for you).

```
## banned-vendor-screen: FAIL. 3 line(s), $21,707.00, paid a vendor on a US banned-vendor list
| confirmed | sample-payments.csv:3 | 2026-01-15 | B&H Foto & Electronics Corp | $15,999.00 | US_DOD_1260H | line names "DJI" ...
| confirmed | sample-payments.csv:2 | 2026-07-01 | Novogene Corporation Inc | $4,828.00 | US_BIOSECURE, US_DOD_1260H | ...
| confirmed | sample-payments.csv:4 | 2025-03-01 | Hikvision USA Inc | $880.00 | US_DOD_1260H, US_FAR_889, US_FCC_COVERED | ...
| review    | sample-payments.csv:5 | 2026-05-01 | BGI LLC | $500,000.00 | ... "bgi" alone is a short name unrelated companies also use ...
```

(`examples/sample-payments.csv`; the Djibouti and dental lines in it produce no hit.)

## GitHub Action

```yaml
- uses: actions/checkout@v4
- uses: Mattbusel/banned-vendor-screen@v0.1.0
  with:
    files: ap/vendor-payments-fy2026.csv
    funds: federal-grant        # decides which rules apply
    fail-on: likely             # confirmed | likely | review | never
    args: --since 2025-12-22    # only lines on or after this day fail the job
```

The action downloads the current lists, installs the binary, writes a table to the job summary and sets `verdict`, `failing-rows`, `failing-usd` and `report` outputs.

## What it checks

| Layer | Example | Level |
|---|---|---|
| Listed name or alias, legal form aside | `BGI Americas Corp` = `BGI Americas Corporation` (1260H, BIOSECURE, a named BGI Group subsidiary) | confirmed |
| Brand of a listed company | `Hikvision USA Inc`, `TP-LINK SYSTEMS INC.` | confirmed |
| Product named on the line, whoever the payee is | `DJI MATRICE 350 RTK`, `DNBSEQ-G400`, `Stereo-seq`, `DS-2CD2143G2`, `UNITREE H1-2`, `AUTEL MAXICHARGER` | confirmed |
| Coined brand inside a longer payee name | `Huawei Device USA`, `BYD Coach & Bus` (with "bus" on the line) | likely |
| GLEIF: a listed company owns 50% or more | `Supercell Oy` (Tencent) | likely |
| Short acronym with nothing on the line to confirm it | `BGI LLC` doing engineering work, `MGI, Inc.` | review |
| Control-only ownership link, one-word name collision, a clause or lawsuit naming the company | `Riot Games`, `52.204-23 prohibition ... Kaspersky`, `US v. Hytera` | review |

Matching works on whole words, never substrings: `DJI` never hits `Djibouti` or `Bemidji`, `ZTE` never hits `Aztec`. Product names that collide with ordinary words need context on the same line: `DJI` needs a drone or camera word (it is also Djibouti's airport code), `Matrice` and `Mavic` need one too (dental matrix bands, bicycle wheels).

Every hit carries the chain (payee, listed entity, parent named by the list, list and listing date), the source URL, and the rules it reaches as of the line's own date:

| Rule | In force | Reaches |
|---|---|---|
| 2 CFR 200.216 (Section 889, grants) | Aug 13 2020 | grant funds buying Huawei, ZTE, Hytera, Hikvision, Dahua gear or services, subsidiaries and affiliates included |
| FAR 52.204-25 (Section 889, contracts) | Aug 13 2019 | agency purchases and contracts |
| American Security Drone Act sec. 1823 | Dec 22 2023 | agency purchases of covered drones |
| American Security Drone Act sec. 1825 | Dec 22 2025 | federal grant and contract funds buying or operating covered drones |
| FY2024 NDAA sec. 805 | Jun 30 2026 direct, Jun 30 2027 through suppliers | DoD contracts with 1260H entities |
| BIOSECURE (FY2026 NDAA sec. 851) | OMB list due Dec 18 2026 | federal contracts, grants and loans for biotech equipment and services |
| UFLPA | each entity's listing date | imports of goods the entity made |
| FCC Covered List | | information only for research funds |

A line dated before the current listing is still reported, with a note that it shows the relationship, not a payment after listing.

## Fails closed

- A list missing from the lists directory, or a list that loads with no names: exit 3. (A sanctions loader once loaded all 207 DoD 1260H entities with empty names and the job went green: [moov-io/watchman#1019](https://github.com/moov-io/watchman/pull/1019).)
- Lists older than `--max-age-days` (default 8) by their `manifest.json`: exit 3. DoD removed WuXi AppTec from 1260H on Nov 10 2026; a list you downloaded in October still has it.
- The GLEIF ownership file missing: exit 3, unless you pass `--no-affiliates` on purpose.
- A column you named that is not in the file: exit 2.

## Inputs

CSV, JSON (an array, or an object with `results`/`data`/`rows`/`items`) or JSON Lines. Nested JSON flattens to dotted keys (`organization.org_name`).

| Preset | Source |
|---|---|
| `auto` (default) | guesses payee, amount, date, award, group and description columns from common open-checkbook headers |
| `usaspending-subawards` | USAspending `spending_by_award` with `subawards: true` |
| `usaspending-transactions` | USAspending `spending_by_transaction` |
| `nih-reporter` | NIH RePORTER v2 `projects/search` (screens title, abstract and terms) |

Override any column with `--vendor`, `--amount`, `--date`, `--award`, `--group`, `--id` and `--text` (repeatable). `--json report.json` writes every hit with its row, line, chain and rules; `--markdown` writes the summary table.

## Reproduce the Nov 20 2026 runs

The pull scripts are in [`examples/`](examples). Each pulls candidates with the source's own keyword search; the tool decides.

| Source | Candidates | Confirmed lines | Dollars on them |
|---|---:|---:|---:|
| NIH RePORTER FY2025-26 projects (`pull_reporter.py`, `--preset nih-reporter`) | 1,869 | 10 | $4,249,852 |
| USAspending transactions FY2024-26 naming a listed product (`pull_transactions.py`) | 3,063 | 151 | $26,609,536 |
| USAspending subawards whose sub-awardee contains a listed brand (`pull_subawards.py`) | 724 | 1 | $168,315 |
| NSF awards (`pull_nsf.py`) | 629 | 2 | $600,000 |
| Washington State agency Amazon purchases FY2024-27 (`pull_wa_amazon.py`) | 375 | 310 | $70,876 |

Lists: us-restricted-parties build of 2026-11-20T15:24:27Z.

## Lists

The lists come from [us-restricted-parties](https://github.com/Mattbusel/us-restricted-parties), rebuilt daily from the Federal Register, eCFR, FCC and DHS as CC BY 4.0 data, with a GLEIF-derived affiliates file for ownership.

## License

MIT. Built by [Tensorust](https://tensorust.com).
