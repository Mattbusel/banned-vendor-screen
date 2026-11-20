//! Small date helpers. Dates are kept as `YYYY-MM-DD` strings so they compare correctly as text.

/// Days since 1970-01-01 for a civil date (proleptic Gregorian).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn valid(y: i64, m: i64, d: i64) -> bool {
    (1900..=2200).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d)
}

fn fmt(y: i64, m: i64, d: i64) -> Option<String> {
    valid(y, m, d).then(|| format!("{y:04}-{m:02}-{d:02}"))
}

/// Parse the date formats public spending files use into `YYYY-MM-DD`.
///
/// Accepts `2025-06-11`, `2025-06-11T00:00:00.000`, `2025/06/11`, `06/11/2025`, `6/11/2025`
/// and `20250611`. Anything else returns `None`: an unknown date is reported as unknown,
/// never guessed.
pub fn parse_date(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let head: String = s
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != 'T')
        .collect();
    let parts: Vec<&str> = head.split(['-', '/', '.']).collect();
    let num = |p: &str| p.parse::<i64>().ok();
    match parts.as_slice() {
        [a, b, c] if a.len() == 4 => fmt(num(a)?, num(b)?, num(c)?),
        [a, b, c] if c.len() == 4 => fmt(num(c)?, num(a)?, num(b)?),
        [a] if a.len() == 8 && a.chars().all(|c| c.is_ascii_digit()) => {
            fmt(num(&a[..4])?, num(&a[4..6])?, num(&a[6..])?)
        }
        _ => None,
    }
}

/// Days since the epoch for a `YYYY-MM-DD` (or longer ISO) string.
pub fn to_days(s: &str) -> Option<i64> {
    let d = parse_date(s)?;
    let y = d[..4].parse().ok()?;
    let m = d[5..7].parse().ok()?;
    let dd = d[8..10].parse().ok()?;
    Some(days_from_civil(y, m, dd))
}

/// Today as days since the epoch (UTC).
pub fn today_days() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    secs.div_euclid(86_400)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_formats() {
        assert_eq!(parse_date("2025-06-11").as_deref(), Some("2025-06-11"));
        assert_eq!(
            parse_date("2025-06-11T00:00:00.000").as_deref(),
            Some("2025-06-11")
        );
        assert_eq!(parse_date("06/11/2025").as_deref(), Some("2025-06-11"));
        assert_eq!(parse_date("6/1/2025").as_deref(), Some("2025-06-01"));
        assert_eq!(parse_date("20250611").as_deref(), Some("2025-06-11"));
        assert_eq!(parse_date("FY2025"), None);
        assert_eq!(parse_date("13/40/2025"), None);
    }

    #[test]
    fn epoch_math() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(
            to_days("2026-11-20").unwrap() - to_days("2026-11-10").unwrap(),
            10
        );
    }
}
