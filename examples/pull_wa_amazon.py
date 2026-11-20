"""Washington State agency Amazon purchases (data.wa.gov, product level: title, brand,
manufacturer, seller) whose brand, maker or title names a listed vendor. Candidates only."""
import json, sys, urllib.parse, urllib.request, csv
OUT = sys.argv[1]
SETS = {"FY24": "e48s-c9au", "FY25": "issh-7ewj", "FY26": "xn6h-92hc", "FY27": "aef4-d4jh"}
BRANDS = ["DJI", "HIKVISION", "DAHUA", "TP-LINK", "TAPO", "HUAWEI", "ZTE", "AUTEL", "UNITREE", "HYTERA", "QUECTEL",
          "KASPERSKY", "HESAI", "EZVIZ", "HILOOK", "INSPUR"]
conds = []
for b in BRANDS:
    conds.append(f"upper(brand) like '%{b}%'")
    conds.append(f"upper(manufacturer) like '%{b}%'")
where = " OR ".join(conds) + " OR upper(title) like '%DJI %' OR upper(title) like '%MAVIC%' OR upper(title) like '%HIKVISION%' OR upper(title) like '%TP-LINK%'"
fields = ["agency_name", "order_date", "payment_date", "title", "brand", "manufacturer", "seller_name", "item_quantity",
          "item_net_total", "item_subtotal", "asin", "item_model_number"]
rows = []
for fy, ds in SETS.items():
    off = 0
    while True:
        q = {"$where": where, "$limit": 50000, "$offset": off}
        url = f"https://data.wa.gov/resource/{ds}.json?" + urllib.parse.urlencode(q)
        d = json.load(urllib.request.urlopen(url, timeout=300))
        for r in d:
            r["fiscal_year"] = fy; r["dataset"] = f"https://data.wa.gov/d/{ds}"
            rows.append(r)
        print(fy, off, len(d), file=sys.stderr)
        if len(d) < 50000: break
        off += 50000
cols = fields + ["fiscal_year", "dataset"]
with open(OUT, "w", newline="", encoding="utf-8") as f:
    w = csv.DictWriter(f, fieldnames=cols, extrasaction="ignore")
    w.writeheader()
    for r in rows: w.writerow({c: r.get(c, "") for c in cols})
print("rows", len(rows))
