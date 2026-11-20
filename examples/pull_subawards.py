"""Pull USAspending subawards (grant subawards and contract subcontracts) whose sub-awardee
name contains a listed vendor's brand. Candidates only; banned-vendor-screen decides."""
import json, sys, time, urllib.request
OUT = sys.argv[1]
TERMS = ["Novogene","Complete Genomics","Innomics","BGI","MGI Tech","MGI Americas","STOmics","DJI","Autel","Hikvision",
         "Dahua","Huawei","Hytera","ZTE","TP-Link","TP Link","Quectel","Unitree","Hesai","RoboSense","Tencent",
         "China Mobile","China Telecom","Kaspersky","SenseTime","WuXi","Contemporary Amperex","SMIC"]
FIELDS = ["Sub-Award ID","Sub-Awardee Name","Sub-Award Date","Sub-Award Amount","Awarding Agency","Awarding Sub Agency",
          "Prime Award ID","Prime Recipient Name","prime_award_recipient_id","Sub-Award Description","sub_award_recipient_id",
          "Sub-Recipient UEI","Prime Award Recipient UEI","prime_award_generated_internal_id"]
GROUPS = {"grants": ["02","03","04","05"], "contracts": ["A","B","C","D"]}
rows = []
for term in TERMS:
    for g, codes in GROUPS.items():
        page = 1
        while True:
            body = {"subawards": True, "limit": 100, "page": page,
                    "filters": {"award_type_codes": codes, "recipient_search_text": [term],
                                "time_period": [{"start_date": "2019-10-01", "end_date": "2026-11-20"}]},
                    "fields": FIELDS, "sort": "Sub-Award Amount", "order": "desc"}
            req = urllib.request.Request("https://api.usaspending.gov/api/v2/search/spending_by_award/",
                                         data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
            for attempt in range(4):
                try:
                    d = json.load(urllib.request.urlopen(req, timeout=120)); break
                except Exception as e:
                    print("retry", term, g, e, file=sys.stderr); time.sleep(5)
            else:
                break
            for r in d.get("results", []):
                r["_term"] = term; r["_group"] = g; rows.append(r)
            print(term, g, page, len(d.get("results", [])), file=sys.stderr)
            if not d.get("page_metadata", {}).get("hasNext") or page >= 20:
                break
            page += 1
json.dump(rows, open(OUT, "w", encoding="utf-8"), indent=1)
print("rows", len(rows))
