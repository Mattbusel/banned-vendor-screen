"""NIH RePORTER v2: FY2025-2026 projects whose title, abstract or terms name a listed vendor's
product or name. Candidates only; banned-vendor-screen decides."""
import json, sys, time, urllib.request
OUT = sys.argv[1]
TERMS = ["DNBSEQ", "DNBseq", "DNB-seq", "MGISEQ", "BGISEQ", "DNBelab", "Stereo-seq", "STOmics", "BGI", "Beijing Genomics Institute", "MGI", "Novogene", "DJI", "Unitree", "Hikvision", "Hesai", "Inspur", "Huawei", "TP-Link", "Quectel"]
rows, seen = [], set()
for term in TERMS:
    offset = 0
    while True:
        body = {"criteria": {"fiscal_years": [2025, 2026],
                             "advanced_text_search": {"operator": "and", "search_field": "projecttitle,abstracttext,terms",
                                                      "search_text": f'"{term}"'}},
                "include_fields": ["ApplId", "ProjectNum", "FiscalYear", "Organization", "AwardAmount", "ProjectStartDate",
                                   "ProjectEndDate", "BudgetStart", "BudgetEnd", "ProjectTitle", "AbstractText", "PrefTerms", "PrincipalInvestigators",
                                   "AgencyIcAdmin", "ActivityCode", "ContactPiName"],
                "offset": offset, "limit": 500}
        req = urllib.request.Request("https://api.reporter.nih.gov/v2/projects/search", data=json.dumps(body).encode(),
                                     headers={"Content-Type": "application/json", "Accept": "application/json"})
        for a in range(4):
            try:
                d = json.load(urllib.request.urlopen(req, timeout=120)); break
            except Exception as e:
                print("retry", term, e, file=sys.stderr); time.sleep(3)
        else:
            break
        res = d.get("results", [])
        total = d.get("meta", {}).get("total", 0)
        for r in res:
            k = (r.get("appl_id"), r.get("fiscal_year"))
            if k in seen: continue
            seen.add(k); r["_term"] = term; rows.append(r)
        print(term, offset, len(res), total, file=sys.stderr)
        offset += 500
        if offset >= total or not res or offset >= 14500: break
        time.sleep(1)
json.dump({"results": rows}, open(OUT, "w", encoding="utf-8"), indent=1)
print("rows", len(rows))
