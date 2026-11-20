//! Vendor families: what a listed company sells, the product names that give it away on a
//! reseller's invoice, and the words that confirm a short acronym really is that company.
//!
//! A payment to "B&H Photo" for a "DJI Matrice 350 RTK" clears every name screen, because the
//! payee is a reseller. The product name is what the rules are about.

use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Sector {
    Genomics,
    Drones,
    VideoSurveillance,
    Telecom,
    Robotics,
    Lidar,
    Software,
    Semiconductors,
    Batteries,
    Vehicles,
    Computing,
    Shipping,
    Other,
}

impl Sector {
    /// Words that, on the same line, confirm a short-acronym name match ("BGI" + "sequencing").
    pub fn words(self) -> &'static [&'static str] {
        match self {
            Sector::Genomics => &[
                "sequenc",
                "genom",
                "dna",
                "rna",
                "library",
                "flow cell",
                "flowcell",
                "reagent",
                "assay",
                "exome",
                "transcriptom",
                "genomic",
                "bioinformatic",
                "pcr",
                "wgs",
            ],
            Sector::Drones => &[
                "drone",
                "uav",
                "uas",
                "unmanned",
                "aircraft",
                "quadcopter",
                "aerial",
                "propeller",
                "mavic",
                "matrice",
            ],
            Sector::VideoSurveillance => &["camera", "cctv", "nvr", "dvr", "surveillance", "video"],
            Sector::Telecom => &[
                "router",
                "switch",
                "network",
                "telecom",
                "phone",
                "radio",
                "modem",
                "module",
                "5g",
                "lte",
                "wireless",
                "wifi",
                "wi-fi",
                "broadband",
                "cellular",
                "circuit",
                "handset",
            ],
            Sector::Robotics => &["robot", "quadruped", "humanoid"],
            Sector::Lidar => &["lidar", "sensor"],
            Sector::Software => &["software", "license", "antivirus", "subscription", "cloud"],
            Sector::Semiconductors => &["wafer", "chip", "foundry", "semiconductor"],
            Sector::Batteries => &["battery", "batteries", "cell"],
            Sector::Vehicles => &[
                "bus", "buses", "coach", "vehicle", "electric", "charger", "transit", "railcar",
                "rail", "battery",
            ],
            Sector::Computing => &[
                "server", "servers", "computer", "storage", "rack", "hardware",
            ],
            Sector::Shipping => &["freight", "shipping", "container", "logistics", "ocean"],
            Sector::Other => &[],
        }
    }
}

#[derive(Debug)]
pub struct ProductRule {
    pub pattern: &'static str,
    /// Listed legal name the product points to (looked up in the loaded lists).
    pub maker: &'static str,
    /// What to call it in the report.
    pub label: &'static str,
    /// True for unmanned aircraft (American Security Drone Act scope).
    pub drone: bool,
    /// Words of which at least one must be on the line too. "DJI" is also the airport code
    /// for Djibouti, "Matrice" is a dental matrix band and "Mavic" a bicycle wheel.
    pub context: &'static [&'static str],
}

#[derive(Debug)]
pub struct Family {
    pub id: &'static str,
    pub label: &'static str,
    pub sector: Sector,
    /// Brand cores (see `normalize::brand_core`) that put a listed entity in this family.
    pub brands: &'static [&'static str],
    /// Section 889 names this family (covers subsidiaries and affiliates too).
    pub sec889: bool,
    pub products: &'static [ProductRule],
}

const fn p(
    pattern: &'static str,
    maker: &'static str,
    label: &'static str,
    drone: bool,
) -> ProductRule {
    ProductRule {
        pattern,
        maker,
        label,
        drone,
        context: &[],
    }
}

const fn pc(
    pattern: &'static str,
    maker: &'static str,
    label: &'static str,
    drone: bool,
    context: &'static [&'static str],
) -> ProductRule {
    ProductRule {
        pattern,
        maker,
        label,
        drone,
        context,
    }
}

const DJI_CONTEXT: &[&str] = &[
    "drone",
    "drones",
    "uas",
    "suas",
    "uav",
    "unmanned",
    "mavic",
    "matrice",
    "zenmuse",
    "mini",
    "air",
    "thermal",
    "camera",
    "gimbal",
    "ronin",
    "osmo",
    "fly",
    "rc",
    "remote",
    "propeller",
    "propellers",
    "battery",
    "agras",
    "avata",
    "inspire",
    "phantom",
    "rtk",
    "dock",
    "aircraft",
    "quadcopter",
];
const DRONE_MODEL_CONTEXT: &[&str] = &[
    "dji",
    "drone",
    "drones",
    "uas",
    "suas",
    "uav",
    "unmanned",
    "thermal",
    "rc",
    "fly",
    "zenmuse",
    "rtk",
    "aircraft",
    "quadcopter",
    "propeller",
];
const AUTEL_CONTEXT: &[&str] = &[
    "drone",
    "drones",
    "uas",
    "uav",
    "evo",
    "charger",
    "chargers",
    "maxicharger",
    "dcfc",
    "charging",
    "scanner",
    "maxisys",
    "diagnostic",
    "robotics",
    "alpha",
    "dispenser",
    "dispensers",
];
const GENOMICS_CONTEXT: &[&str] = &[
    "sequenc",
    "genom",
    "dna",
    "rna",
    "transcriptom",
    "stereo",
    "library",
    "libraries",
    "exome",
    "wgs",
    "omics",
    "spatial",
    "reagent",
    "assay",
];
const ZTE_CONTEXT: &[&str] = &[
    "phone",
    "phones",
    "router",
    "modem",
    "hotspot",
    "handset",
    "network",
    "telecom",
    "lte",
    "5g",
    "equipment",
    "device",
    "devices",
    "mobile",
];

pub static FAMILIES: &[Family] = &[
    Family {
        id: "bgi-mgi",
        label: "BGI / MGI / Complete Genomics",
        sector: Sector::Genomics,
        brands: &[
            "bgi",
            "bgi genomics",
            "bgi group",
            "innomics",
            "stomics",
            "complete genomics",
            "mgi",
            "mgi tech",
        ],
        sec889: false,
        products: &[
            p(
                r"(?i)\bDNBSEQ",
                "MGI Tech Co., Ltd.",
                "DNBSEQ sequencer or flow cell (MGI)",
                false,
            ),
            p(
                r"(?i)\bBGISEQ",
                "MGI Tech Co., Ltd.",
                "BGISEQ sequencer (MGI)",
                false,
            ),
            p(
                r"(?i)\bMGISEQ",
                "MGI Tech Co., Ltd.",
                "MGISEQ sequencer (MGI)",
                false,
            ),
            p(
                r"(?i)\bDNBelab",
                "MGI Tech Co., Ltd.",
                "DNBelab instrument (MGI)",
                false,
            ),
            p(
                r"(?i)\bMGIEasy",
                "MGI Tech Co., Ltd.",
                "MGIEasy reagent kit (MGI)",
                false,
            ),
            p(
                r"(?i)\bStereo-seq\b",
                "STomics Americas, Ltd.",
                "Stereo-seq spatial kit (STOmics, BGI)",
                false,
            ),
            p(
                r"(?i)\bSTOmics\b",
                "STomics Americas, Ltd.",
                "STOmics spatial platform (BGI)",
                false,
            ),
            pc(
                r"\bBGI\b",
                "BGI Genomics Co., Ltd.",
                "BGI (sequencing provider)",
                false,
                GENOMICS_CONTEXT,
            ),
        ],
    },
    Family {
        id: "novogene",
        label: "Novogene",
        sector: Sector::Genomics,
        brands: &["novogene"],
        sec889: false,
        products: &[p(
            r"(?i)\bNovogene\b",
            "Novogene Company Limited",
            "Novogene (sequencing provider)",
            false,
        )],
    },
    Family {
        id: "dji",
        label: "DJI",
        sector: Sector::Drones,
        brands: &["dji"],
        sec889: false,
        products: &[
            pc(
                r"\bDJI\b",
                "Shenzhen DJI Technology Co., Ltd.",
                "DJI product",
                true,
                DJI_CONTEXT,
            ),
            pc(
                r"(?i)\bMavic\b",
                "Shenzhen DJI Technology Co., Ltd.",
                "DJI Mavic drone",
                true,
                DRONE_MODEL_CONTEXT,
            ),
            pc(
                r"(?i)\bMatrice\b",
                "Shenzhen DJI Technology Co., Ltd.",
                "DJI Matrice drone",
                true,
                DRONE_MODEL_CONTEXT,
            ),
            p(
                r"(?i)\bZenmuse\b",
                "Shenzhen DJI Technology Co., Ltd.",
                "DJI Zenmuse drone camera",
                true,
            ),
            pc(
                r"(?i)\bAgras\b",
                "Shenzhen DJI Technology Co., Ltd.",
                "DJI Agras spray drone",
                true,
                DRONE_MODEL_CONTEXT,
            ),
            p(
                r"(?i)\bPhantom\s?4\b",
                "Shenzhen DJI Technology Co., Ltd.",
                "DJI Phantom 4 drone",
                true,
            ),
            pc(
                r"(?i)\bAvata\b",
                "Shenzhen DJI Technology Co., Ltd.",
                "DJI Avata drone",
                true,
                DRONE_MODEL_CONTEXT,
            ),
            pc(
                r"(?i)\bOsmo\b",
                "Shenzhen DJI Technology Co., Ltd.",
                "DJI Osmo camera",
                false,
                &["dji", "camera", "gimbal", "pocket", "action"],
            ),
        ],
    },
    Family {
        id: "autel",
        label: "Autel",
        sector: Sector::Drones,
        brands: &["autel", "autel robotics", "autel intelligent"],
        sec889: false,
        products: &[
            p(
                r"(?i)\bAutel\s+(Robotics|EVO)\b",
                "Autel Robotics Co., Ltd.",
                "Autel drone",
                true,
            ),
            pc(
                r"(?i)\bAutel\b",
                "Autel Intelligent Technology Corp., Ltd.",
                "Autel product",
                false,
                AUTEL_CONTEXT,
            ),
        ],
    },
    Family {
        id: "hikvision",
        label: "Hikvision",
        sector: Sector::VideoSurveillance,
        brands: &["hikvision"],
        sec889: true,
        products: &[
            p(
                r"(?i)\bHikvision\b",
                "Hangzhou Hikvision Digital Technology Co., Ltd.",
                "Hikvision product",
                false,
            ),
            p(
                r"\bDS-2CD\w*",
                "Hangzhou Hikvision Digital Technology Co., Ltd.",
                "Hikvision DS-2CD camera",
                false,
            ),
            p(
                r"\bDS-[79]\d{3}\w*",
                "Hangzhou Hikvision Digital Technology Co., Ltd.",
                "Hikvision DS-7/9 recorder",
                false,
            ),
            p(
                r"(?i)\bHiLook\b",
                "Hangzhou Hikvision Digital Technology Co., Ltd.",
                "HiLook (Hikvision brand)",
                false,
            ),
        ],
    },
    Family {
        id: "dahua",
        label: "Dahua",
        sector: Sector::VideoSurveillance,
        brands: &["dahua"],
        sec889: true,
        products: &[
            p(
                r"(?i)\bDahua\b",
                "Zhejiang Dahua Technology Co., Ltd.",
                "Dahua product",
                false,
            ),
            p(
                r"\bIPC-H[DF]W\w*",
                "Zhejiang Dahua Technology Co., Ltd.",
                "Dahua IPC camera",
                false,
            ),
        ],
    },
    Family {
        id: "huawei",
        label: "Huawei",
        sector: Sector::Telecom,
        brands: &["huawei"],
        sec889: true,
        products: &[p(
            r"(?i)\bHuawei\b",
            "Huawei Technologies Co., Ltd.",
            "Huawei product",
            false,
        )],
    },
    Family {
        id: "zte",
        label: "ZTE",
        sector: Sector::Telecom,
        brands: &["zte"],
        sec889: true,
        products: &[pc(
            r"\bZTE\b",
            "ZTE Corporation",
            "ZTE product",
            false,
            ZTE_CONTEXT,
        )],
    },
    Family {
        id: "hytera",
        label: "Hytera",
        sector: Sector::Telecom,
        brands: &["hytera"],
        sec889: true,
        products: &[p(
            r"(?i)\bHytera\b",
            "Hytera Communications Corporation",
            "Hytera radio",
            false,
        )],
    },
    Family {
        id: "tp-link",
        label: "TP-Link",
        sector: Sector::Telecom,
        brands: &["tp link"],
        sec889: false,
        products: &[
            p(
                r"(?i)\bTP-?Link\b",
                "TP-Link Technologies Co., Ltd.",
                "TP-Link network gear",
                false,
            ),
            p(
                r"(?i)\bOmada\b",
                "TP-Link Technologies Co., Ltd.",
                "TP-Link Omada network gear",
                false,
            ),
            p(
                r"(?i)\bTapo\b",
                "TP-Link Technologies Co., Ltd.",
                "Tapo (TP-Link brand)",
                false,
            ),
        ],
    },
    Family {
        id: "quectel",
        label: "Quectel",
        sector: Sector::Telecom,
        brands: &["quectel"],
        sec889: false,
        products: &[p(
            r"(?i)\bQuectel\b",
            "Quectel Wireless Solutions Co., Ltd.",
            "Quectel cellular module",
            false,
        )],
    },
    Family {
        id: "china-telecom-carriers",
        label: "China Mobile / China Telecom",
        sector: Sector::Telecom,
        brands: &[
            "china mobile",
            "china telecom",
            "china unicom",
            "china mobile comm",
        ],
        sec889: false,
        products: &[],
    },
    Family {
        id: "unitree",
        label: "Unitree",
        sector: Sector::Robotics,
        brands: &["unitree", "yushu"],
        sec889: false,
        products: &[p(
            r"(?i)\bUnitree\b",
            "Hangzhou Yushu Technology Co., Ltd.",
            "Unitree robot",
            false,
        )],
    },
    Family {
        id: "hesai",
        label: "Hesai",
        sector: Sector::Lidar,
        brands: &["hesai"],
        sec889: false,
        products: &[p(r"(?i)\bHesai\b", "Hesai Group", "Hesai lidar", false)],
    },
    Family {
        id: "robosense",
        label: "RoboSense",
        sector: Sector::Lidar,
        brands: &["robosense"],
        sec889: false,
        products: &[p(
            r"(?i)\bRoboSense\b",
            "RoboSense Technology Co., Ltd.",
            "RoboSense lidar",
            false,
        )],
    },
    Family {
        id: "kaspersky",
        label: "Kaspersky",
        sector: Sector::Software,
        brands: &["kaspersky", "ao kaspersky lab", "kaspersky lab"],
        sec889: false,
        products: &[p(
            r"(?i)\bKaspersky\b",
            "Kaspersky Lab, Inc",
            "Kaspersky software",
            false,
        )],
    },
    Family {
        id: "tencent",
        label: "Tencent",
        sector: Sector::Software,
        brands: &["tencent"],
        sec889: false,
        products: &[p(
            r"(?i)\bTencent\b",
            "Tencent Holdings Limited",
            "Tencent service",
            false,
        )],
    },
    Family {
        id: "catl",
        label: "CATL",
        sector: Sector::Batteries,
        brands: &["catl", "contemporary amperex"],
        sec889: false,
        products: &[],
    },
    Family {
        id: "smic",
        label: "SMIC",
        sector: Sector::Semiconductors,
        brands: &["smic", "semiconductor manufacturing"],
        sec889: false,
        products: &[],
    },
    Family {
        id: "byd",
        label: "BYD",
        sector: Sector::Vehicles,
        brands: &["byd"],
        sec889: false,
        products: &[],
    },
    Family {
        id: "crrc",
        label: "CRRC",
        sector: Sector::Vehicles,
        brands: &["crrc"],
        sec889: false,
        products: &[],
    },
    Family {
        id: "inspur",
        label: "Inspur",
        sector: Sector::Computing,
        brands: &["inspur"],
        sec889: false,
        products: &[p(
            r"(?i)Inspur",
            "Inspur Group Co., Ltd.",
            "Inspur server",
            false,
        )],
    },
    Family {
        id: "cosco",
        label: "COSCO Shipping",
        sector: Sector::Shipping,
        brands: &["cosco shipping", "cosco"],
        sec889: false,
        products: &[],
    },
    Family {
        id: "eve-energy",
        label: "EVE Energy",
        sector: Sector::Batteries,
        brands: &["eve energy"],
        sec889: false,
        products: &[],
    },
    Family {
        id: "sensetime",
        label: "SenseTime",
        sector: Sector::Software,
        brands: &["sensetime"],
        sec889: false,
        products: &[p(
            r"(?i)\bSenseTime\b",
            "SenseTime Group, Inc.",
            "SenseTime software",
            false,
        )],
    },
];

/// The family a listed name belongs to, by its brand core or any word run of its core.
pub fn family_of(names: &[String]) -> Option<&'static Family> {
    for n in names {
        let b = crate::normalize::brand_core(n);
        let toks = crate::normalize::core_tokens(n);
        for f in FAMILIES {
            for br in f.brands {
                let bt: Vec<String> = br.split(' ').map(str::to_string).collect();
                if b == *br || crate::normalize::contains_words(&toks, &bt) {
                    return Some(f);
                }
            }
        }
    }
    None
}

pub struct CompiledProduct {
    pub re: Regex,
    pub rule: &'static ProductRule,
    pub family: &'static Family,
}

pub fn compiled_products() -> Vec<CompiledProduct> {
    let mut out = Vec::new();
    for f in FAMILIES {
        for r in f.products {
            out.push(CompiledProduct {
                re: Regex::new(r.pattern).expect("product pattern compiles"),
                rule: r,
                family: f,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_resolve_from_listed_names() {
        let f = family_of(&["Shenzhen DJI Technology Co., Ltd.".to_string()]).unwrap();
        assert_eq!(f.id, "dji");
        let f = family_of(&["Complete Genomics, Inc.".to_string()]).unwrap();
        assert_eq!(f.id, "bgi-mgi");
        let f = family_of(&["Hangzhou Hikvision Digital Technology Company".to_string()]).unwrap();
        assert!(f.sec889);
        assert!(family_of(&["Xinjiang Example Cotton Co.".to_string()]).is_none());
    }

    #[test]
    fn product_patterns_compile_and_hit() {
        let ps = compiled_products();
        let hit = |s: &str| {
            let lower = s.to_lowercase();
            ps.iter()
                .find(|p| {
                    p.re.is_match(s)
                        && (p.rule.context.is_empty()
                            || p.rule
                                .context
                                .iter()
                                .any(|w| crate::normalize::has_word(&lower, w)))
                })
                .map(|p| p.family.id)
        };
        assert_eq!(hit("DJI MATRICE 350 RTK COMBO"), Some("dji"));
        assert_eq!(
            hit("DNBSEQ-G400RS HIGH-THROUGHPUT FLOW CELL"),
            Some("bgi-mgi")
        );
        assert_eq!(hit("DS-2CD2143G2-I 4MP dome"), Some("hikvision"));
        assert_eq!(hit("Employment agency of Djibouti"), None);
        assert_eq!(hit("Aztec Door & Hardware"), None);
        assert_eq!(hit("TANKER VOYAGE CHARTER MAL-UAE-DJI"), None);
        assert_eq!(hit("MATRICE DNTL GRAY 100S"), None);
        assert_eq!(hit("4565675553!MATRICE 30T KIT THERMAL"), Some("dji"));
    }
}
