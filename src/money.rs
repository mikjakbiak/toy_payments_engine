pub const SCALE: u64 = 10_000;

pub fn amount_from_str(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty amount".into());
    }

    let parts: Vec<&str> = s.split('.').collect();
    match parts.as_slice() {
        [whole] => {
            let whole: u64 = whole.parse().map_err(|_| format!("invalid amount: {s}"))?;
            whole
                .checked_mul(SCALE)
                .ok_or_else(|| format!("amount overflow: {s}"))
        }
        [whole, frac] => {
            if frac.len() > 4 {
                return Err(format!("amount has more than 4 decimal places: {s}"));
            }
            let whole: u64 = whole.parse().map_err(|_| format!("invalid amount: {s}"))?;
            let frac_padded = format!("{:0<4}", frac);
            let frac_val: u64 = frac_padded
                .parse()
                .map_err(|_| format!("invalid amount: {s}"))?;
            whole
                .checked_mul(SCALE)
                .and_then(|v| v.checked_add(frac_val))
                .ok_or_else(|| format!("amount overflow: {s}"))
        }
        _ => Err(format!("invalid amount: {s}")),
    }
}

pub fn amount_to_string(amount: u64) -> String {
    let whole = amount / SCALE;
    let frac = amount % SCALE;

    if frac == 0 {
        return whole.to_string();
    }

    let mut frac_str = format!("{:04}", frac);
    while frac_str.ends_with('0') {
        frac_str.pop();
    }

    format!("{whole}.{frac_str}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_amount_examples() {
        assert_eq!(amount_from_str("12.2").unwrap(), 122_000);
        assert_eq!(amount_from_str("2").unwrap(), 20_000);
        assert_eq!(amount_from_str("30").unwrap(), 300_000);
        assert_eq!(amount_from_str("1.2345").unwrap(), 12_345);
    }

    #[test]
    fn formats_amount_examples() {
        assert_eq!(amount_to_string(500_000), "50");
        assert_eq!(amount_to_string(146_200), "14.62");
        assert_eq!(amount_to_string(122_000), "12.2");
        assert_eq!(amount_to_string(12_345), "1.2345");
    }
}
