//! Name normalization. Matching works on whole words, never on substrings: "zte" must not
//! match "Aztec", and "dji" must not match "Djibouti".

const LEGAL: &[&str] = &[
    "inc",
    "incorporated",
    "corp",
    "corporation",
    "co",
    "company",
    "llc",
    "ltd",
    "limited",
    "plc",
    "lp",
    "llp",
    "sa",
    "ag",
    "gmbh",
    "bv",
    "pte",
    "pvt",
    "sarl",
    "srl",
    "slu",
    "sl",
    "spa",
    "nv",
    "oy",
    "ab",
    "kk",
    "sas",
    "kg",
    "the",
];

const GEO: &[&str] = &["usa", "us", "america", "americas", "intl", "international"];

/// Chinese place names that lead many listed legal names ("Shenzhen DJI Technology").
pub const PLACES: &[&str] = &[
    "shenzhen",
    "hangzhou",
    "zhejiang",
    "beijing",
    "shanghai",
    "guangzhou",
    "jiangsu",
    "chengdu",
    "tianjin",
    "hefei",
    "nanjing",
    "suzhou",
    "wuhan",
    "qingdao",
    "xiamen",
];

/// Words that describe a company rather than name it.
pub const GENERIC: &[&str] = &[
    "technology",
    "technologies",
    "tech",
    "group",
    "holdings",
    "holding",
    "digital",
    "wireless",
    "solutions",
    "communications",
    "communication",
    "electronics",
    "systems",
    "intelligent",
];

/// Lowercase word tokens. `&` becomes "and", every other non-alphanumeric character splits.
/// Dotted initials join up first ("L.L.C." -> "llc", "U.S." -> "us"), and other single
/// letters stay, so "S & S Electronics" never shrinks to "and electronics".
pub fn tokens(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_alphanumeric() {
            cur.extend(ch.to_lowercase());
            continue;
        }
        // A dot right after a single letter that itself follows a boundary or another
        // initial ("L.L.C."): keep building the same token.
        if ch == '.' && !cur.is_empty() {
            let run = chars[..i]
                .iter()
                .rev()
                .take_while(|c| c.is_alphanumeric())
                .count();
            let before = i.checked_sub(run + 1).map(|j| chars[j]);
            let starts_at_boundary = before.map_or(true, |b| b == '.' || !b.is_alphanumeric());
            let next_ends = chars.get(i + 1).map_or(true, |c| !c.is_alphanumeric());
            let next_single = chars.get(i + 1).is_some_and(|c| c.is_alphabetic())
                && chars.get(i + 2).map_or(true, |c| !c.is_alphanumeric());
            if run == 1 && starts_at_boundary && (next_single || next_ends) {
                continue;
            }
        }
        if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        if ch == '&' {
            out.push("and".to_string());
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn strip_north_america(t: &mut Vec<String>) {
    let mut i = 0;
    while i + 1 < t.len() {
        if t[i] == "north" && (t[i + 1] == "america" || t[i + 1] == "americas") {
            t.drain(i..i + 2);
        } else {
            i += 1;
        }
    }
}

/// The name with legal forms and US/geography words removed. "Hikvision USA, Inc." -> "hikvision".
pub fn core_tokens(s: &str) -> Vec<String> {
    let mut t = tokens(s);
    strip_north_america(&mut t);
    t.retain(|w| !LEGAL.contains(&w.as_str()) && !GEO.contains(&w.as_str()));
    t
}

/// The name with legal forms removed but geography kept: "BGI Americas Corporation" ->
/// "bgi americas". An exact match on this is a match on the listed name itself.
pub fn legal_core(s: &str) -> String {
    let mut t = tokens(s);
    t.retain(|w| !LEGAL.contains(&w.as_str()));
    t.join(" ")
}

pub fn core(s: &str) -> String {
    core_tokens(s).join(" ")
}

/// The brand: the core without leading place names and generic descriptors.
/// "Hangzhou Hikvision Digital Technology Co., Ltd." -> "hikvision". Falls back to the core.
pub fn brand_core(s: &str) -> String {
    let c = core_tokens(s);
    let b: Vec<&String> = c
        .iter()
        .filter(|w| !PLACES.contains(&w.as_str()) && !GENERIC.contains(&w.as_str()))
        .collect();
    if b.is_empty() {
        c.join(" ")
    } else {
        b.into_iter().cloned().collect::<Vec<_>>().join(" ")
    }
}

/// True when `word` starts a word in `line_lower` ("sequenc" hits "sequencing"). Words of four
/// letters or fewer must match a whole word, so "rna" does not hit "international"; a word
/// with a space or hyphen must match as a phrase.
pub fn has_word(line_lower: &str, word: &str) -> bool {
    let toks: Vec<&str> = line_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    if word.contains(' ') || word.contains('-') {
        let spaced = format!(" {} ", toks.join(" "));
        return spaced.contains(&format!(" {} ", word.replace('-', " ")));
    }
    if word.chars().count() <= 4 {
        toks.contains(&word)
    } else {
        toks.iter().any(|t| t.starts_with(word))
    }
}

/// True when `needle` occurs in `hay` as a run of whole words.
pub fn contains_words(hay: &[String], needle: &[String]) -> bool {
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| w == needle)
}

/// A one-word key of four characters or fewer ("bgi", "dji", "zte", "smic") is shared by
/// unrelated companies, so a match on it alone needs corroboration.
pub fn is_short_key(key: &str) -> bool {
    !key.contains(' ') && key.chars().count() <= 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_and_geo_words_go() {
        assert_eq!(core("Hikvision USA, Inc."), "hikvision");
        assert_eq!(core("NOVOGENE CORPORATION INC"), "novogene");
        assert_eq!(core("China Telecom (Americas) Corp"), "china telecom");
        assert_eq!(core("BGI Americas Corporation"), "bgi");
        assert_eq!(core("Acme Robotics L.L.C."), "acme robotics");
        assert_eq!(core("S & S Electronics LLC"), "s and s electronics");
        assert_eq!(core("M&S Electronics"), "m and s electronics");
        assert_eq!(core("Acme U.S.A. Inc."), "acme");
        assert_eq!(legal_core("BGI Americas Corporation"), "bgi americas");
        assert_eq!(core("DJI Technology North America"), "dji technology");
    }

    #[test]
    fn brand_strips_places_and_descriptors() {
        assert_eq!(
            brand_core("Hangzhou Hikvision Digital Technology Co., Ltd."),
            "hikvision"
        );
        assert_eq!(brand_core("Shenzhen DJI Technology Co., Ltd."), "dji");
        assert_eq!(
            brand_core("Quectel Wireless Solutions Co., Ltd."),
            "quectel"
        );
        assert_eq!(brand_core("TP-Link Technologies Co., Ltd."), "tp link");
    }

    #[test]
    fn whole_words_only() {
        let hay = core_tokens("Aztec Municipal School District");
        assert!(!contains_words(&hay, &["zte".to_string()]));
        let hay = core_tokens("Employment Agency of Djibouti");
        assert!(!contains_words(&hay, &["dji".to_string()]));
        let hay = core_tokens("Huawei Device USA Inc");
        assert!(contains_words(&hay, &["huawei".to_string()]));
    }
}
