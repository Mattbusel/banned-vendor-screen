"""NSF Award Search API: awards whose title or abstract names a listed vendor's product.
Candidates only; banned-vendor-screen decides."""
import json, sys, time, urllib.parse, urllib.request
OUT = sys.argv[1]
KEYS = ["DJI", "Mavic", "Matrice", "Zenmuse", "Unitree", "Hikvision", "DNBSEQ", "MGISEQ", "Novogene", "Stereo-seq",
        "STOmics", "Hesai", "Autel", "TP-Link", "Quectel", "Huawei"]
FIELDS = "id,title,abstractText,awardeeName,fundsObligatedAmt,estimatedTotalAmt,startDate,expDate,date,piFirstName,piLastName,poName,fundProgramName,awardeeStateCode,agency"
rows, seen = [], set()
for k in KEYS:
    off = 1
    while True:
        q = {"keyword": k, "printFields": FIELDS, "offset": off, "rpp": 25, "dateStart": "01/01/2023"}
        url = "https://api.nsf.gov/services/v1/awards.json?" + urllib.parse.urlencode(q)
        for a in range(4):
            try:
                d = json.load(urllib.request.urlopen(url, timeout=120)); break
            except Exception as e:
                print("retry", k, e, file=sys.stderr); time.sleep(3)
        else:
            break
        aw = d.get("response", {}).get("award", [])
        for r in aw:
            if r["id"] in seen: continue
            seen.add(r["id"]); r["_key"] = k; r["url"] = f"https://www.nsf.gov/awardsearch/showAward?AWD_ID={r['id']}"
            rows.append(r)
        print(k, off, len(aw), file=sys.stderr)
        if len(aw) < 25 or off > 500: break
        off += 25
        time.sleep(0.5)
json.dump({"results": rows}, open(OUT, "w", encoding="utf-8"), indent=1)
print("rows", len(rows))
