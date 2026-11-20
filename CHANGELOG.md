# Changelog

## 0.1.0 (2026-11-20)

First release.

Why: federal research money already pays vendors the US has banned, and the usual check (payee names against OFAC and the Consolidated Screening List) cannot see it. Four of the five lists that reach research money are in neither, the payee is often a reseller, and listed companies' US subsidiaries are rarely listed by name.

- Screens payment lines, subawards and award text against DoD 1260H, Section 889, the FCC Covered List, UFLPA and BIOSECURE from the us-restricted-parties feed, with named subsidiaries, GLEIF owners (50%+ owned = likely, control-only = review) and product names (DNBSEQ, Stereo-seq, DJI Mavic/Matrice/Zenmuse, Hikvision DS-2CD, Unitree, Autel, TP-Link, Hytera, Quectel, Hesai and more).
- Three levels (confirmed, likely, review); `--fail-on` picks the one that fails the run. Exit 1 on a covered vendor.
- Whole-word matching; short acronyms (BGI, DJI, ZTE, MGI) need a sector word or product on the same line; colliding product words (DJI the airport code, Matrice the dental band) need context; clause, lawsuit and rip-and-replace lines drop to review.
- Rules per hit as of the line's date: 2 CFR 200.216, FAR 52.204-25, ASDA secs. 1823 and 1825, FY2024 NDAA sec. 805, BIOSECURE, UFLPA, FCC Covered List.
- Fails closed: a missing, empty or stale list (default 8 days) exits 3.
- Presets for USAspending subawards and transactions and NIH RePORTER; CSV, JSON and JSON Lines; JSON report, Markdown summary, GitHub step summary and outputs.
- GitHub Action (composite) that fetches the current lists and installs a prebuilt binary.
