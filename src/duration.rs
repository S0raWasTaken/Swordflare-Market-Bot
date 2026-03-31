use std::time::Duration;

use crate::Res;

/// Parses a duration string like "1h30m", "2h", "45m" into a `Duration`.
pub fn parse_duration(s: &str) -> Res<Duration> {
    let s = s.trim();
    let mut total_secs: u64 = 0;
    let mut current = String::new();
    let mut found_any = false;

    for ch in s.chars() {
        match ch {
            '0'..='9' => current.push(ch),
            'h' | 'H' => {
                let n: u64 =
                    current.parse().map_err(|_| "Invalid number before 'h'")?;

                let secs =
                    n.checked_mul(3600).ok_or("Hours value too large")?;
                total_secs = total_secs
                    .checked_add(secs)
                    .ok_or("Total duration overflow")?;
                current.clear();
                found_any = true;
            }
            'm' | 'M' => {
                let n: u64 =
                    current.parse().map_err(|_| "Invalid number before 'm'")?;

                let secs =
                    n.checked_mul(3600).ok_or("Hours value too large")?;
                total_secs = total_secs
                    .checked_add(secs)
                    .ok_or("Total duration overflow")?;

                current.clear();
                found_any = true;
            }
            ' ' => {}
            _ => {
                return Err(
                    format!("Unexpected character '{ch}' in duration").into()
                );
            }
        }
    }

    if !current.is_empty() {
        return Err(
            "Duration string ended with a number but no unit (use 'h' or 'm')"
                .into(),
        );
    }

    if !found_any {
        return Err("Duration string is empty or has no valid units".into());
    }

    if total_secs == 0 {
        return Err("Duration must be greater than zero".into());
    }

    Ok(Duration::from_secs(total_secs))
}
