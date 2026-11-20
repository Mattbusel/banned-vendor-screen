"""USAspending spending_by_transaction: contract and grant transactions whose description names a
listed vendor's product (resellers included). Candidates only; banned-vendor-screen decides."""
import json, sys, time, urllib.request
OUT = sys.argv[1]
KEYS = ["DJI", "Mavic", "Matrice", "Zenmuse", "Agras", "DNBSEQ", "MGISEQ", "BGISEQ", "Hikvision", "Dahua", "Novogene",
        "Complete Genomics", "Autel", "Unitree", "Hesai", "Inspur", "TP-Link", "Quectel", "Hytera", "Huawei", "Kaspersky"]
FIELDS = ["Award ID", "Recipient Name", "Action Date", "Transaction Amount", "Transaction Description", "Awarding Agency",
          "Awarding Sub Agency", "internal_id", "generated_internal_id", "Recipient UEI", "Funding Sub Agency"]
GROUPS = {"contracts": ["A", "B", "C", "D"], "grants": ["02", "03", "04", "05"]}
rows = []
for k in KEYS:
    for g, codes in GROUPS.items():
        page = 1
        while True:
            body = {"filters": {"keywords": [k], "award_type_codes": codes,
                                "time_period": [{"start_date": "2023-10-01", "end_date": "2026-11-20"}]},
                    "fields": FIELDS, "limit": 100, "page": page, "sort": "Transaction Amount", "order": "desc"}
            req = urllib.request.Request("https://api.usaspending.gov/api/v2/search/spending_by_transaction/",
                                         data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
            for a in range(4):
                try:
                    d = json.load(urllib.request.urlopen(req, timeout=180)); break
                except Exception as e:
                    print("retry", k, g, e, file=sys.stderr); time.sleep(5)
            else:
                break
            res = d.get("results", [])
            for r in res:
                r["_key"] = k; r["_group"] = g; rows.append(r)
            print(k, g, page, len(res), file=sys.stderr)
            if not d.get("page_metadata", {}).get("hasNext") or page >= 10:
                break
            page += 1
json.dump(rows, open(OUT, "w", encoding="utf-8"), indent=1)
print("rows", len(rows))
