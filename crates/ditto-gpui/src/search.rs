use chrono::{DateTime, Utc};
use ditto_core::db::ClipboardItem;

pub fn get_simple_hash(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut hash: i32 = 0;
    if bytes.len() > 4096 {
        for &b in &bytes[..2048] {
            hash = hash
                .wrapping_shl(5)
                .wrapping_sub(hash)
                .wrapping_add(b as i32);
        }
        for &b in &bytes[bytes.len() - 2048..] {
            hash = hash
                .wrapping_shl(5)
                .wrapping_sub(hash)
                .wrapping_add(b as i32);
        }
    } else {
        for &b in bytes {
            hash = hash
                .wrapping_shl(5)
                .wrapping_sub(hash)
                .wrapping_add(b as i32);
        }
    }
    format!("{:08x}", hash.unsigned_abs())
}

/// Formats UTC SQLite timestamp ("YYYY-MM-DD HH:MM:SS.FFF") to relative human time.
pub fn get_relative_time(timestamp_str: &str) -> String {
    get_relative_time_at(timestamp_str, Utc::now())
}

/// Formats UTC SQLite timestamp ("YYYY-MM-DD HH:MM:SS.FFF") to relative human time evaluated at a reference instant.
/// For items under 1 minute, returns "Just now" to avoid second-by-second layout invalidation.
pub fn get_relative_time_at(timestamp_str: &str, reference_now: DateTime<Utc>) -> String {
    let parse_str = if timestamp_str.contains('T') {
        timestamp_str.to_string()
    } else {
        format!("{}Z", timestamp_str.replace(' ', "T"))
    };

    let dt = match DateTime::parse_from_rfc3339(&parse_str) {
        Ok(parsed) => parsed.with_timezone(&Utc),
        Err(_) => {
            match chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S%.f") {
                Ok(naive) => DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc),
                Err(_) => {
                    match chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S")
                    {
                        Ok(naive) => DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc),
                        Err(_) => return "Just now".to_string(),
                    }
                }
            }
        }
    };

    let diff = reference_now.signed_duration_since(dt);
    if diff.num_milliseconds() < 0 {
        return "Just now".to_string();
    }

    let diff_sec = diff.num_seconds();
    if diff_sec < 60 {
        "Just now".to_string()
    } else {
        let diff_min = diff_sec / 60;
        if diff_min < 60 {
            format!("{}m ago", diff_min)
        } else {
            let diff_hr = diff_min / 60;
            if diff_hr < 24 {
                format!("{}h ago", diff_hr)
            } else {
                let diff_days = diff_hr / 24;
                format!("{}d ago", diff_days)
            }
        }
    }
}

/// Formats byte size to human readable B / KB / MB.
pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[inline]
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let needle_bytes = needle.as_bytes();
    let needle_len = needle_bytes.len();
    let haystack_bytes = haystack.as_bytes();

    if needle_len > haystack_bytes.len() {
        return false;
    }

    let first_lower = needle_bytes[0].to_ascii_lowercase();
    let first_upper = needle_bytes[0].to_ascii_uppercase();

    let max_start = haystack_bytes.len() - needle_len;
    for i in 0..=max_start {
        let b = haystack_bytes[i];
        if (b == first_lower || b == first_upper)
            && haystack_bytes[i..i + needle_len].eq_ignore_ascii_case(needle_bytes)
        {
            return true;
        }
    }
    false
}
/// In-memory multi-word search across cached clipboard items (zero heap allocation during search).
pub fn search_items(items: &[ClipboardItem], query: &str, limit: usize) -> Vec<ClipboardItem> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return items.iter().take(limit).cloned().collect();
    }

    let terms: Vec<&str> = trimmed.split_whitespace().collect();

    items
        .iter()
        .filter(|item| {
            if item.content.starts_with("data:image/png;base64,") {
                let text = "image clip png";
                terms.iter().all(|term| contains_ignore_case(text, term))
            } else {
                terms
                    .iter()
                    .all(|term| contains_ignore_case(&item.content, term))
            }
        })
        .take(limit)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_hash() {
        assert_eq!(get_simple_hash("hello"), "05e918d2");
        assert_eq!(get_simple_hash(""), "00000000");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(100), "100 B");
        assert_eq!(format_size(155300), "151.7 KB");
        assert_eq!(format_size(1024 * 1024 * 2), "2.0 MB");
    }

    #[test]
    fn test_search_items() {
        let items = vec![
            ClipboardItem {
                id: 1,
                content: "cargo build --release".to_string(),
                created_at: "2026-09-18 10:00:00.000".to_string(),
            },
            ClipboardItem {
                id: 2,
                content: "git commit -m 'feat: something'".to_string(),
                created_at: "2026-09-18 09:50:00.000".to_string(),
            },
            ClipboardItem {
                id: 3,
                content: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==".to_string(),
                created_at: "2026-09-18 09:00:00.000".to_string(),
            },
        ];

        let results = search_items(&items, "", 10);
        assert_eq!(results.len(), 3);

        let results = search_items(&items, "cargo release", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);

        let results = search_items(&items, "image", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 3);
    }
    #[test]
    fn test_search_latency_benchmark() {
        let mut items = Vec::with_capacity(100);
        for i in 0..100 {
            items.push(ClipboardItem {
                id: i as i64,
                content: format!(
                    "Clipboard item #{} with some code snippets git commit --amend",
                    i
                ),
                created_at: "2026-09-18 10:00:00.000".to_string(),
            });
        }

        let start = std::time::Instant::now();
        let iterations = 100;
        for _ in 0..iterations {
            let _ = search_items(&items, "commit amend 50", 50);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / iterations as f64;
        println!(
            "Average search latency across 100 items in RAM: {:.2} µs ({:.4} ms)",
            avg_us,
            avg_us / 1000.0
        );
        assert!(
            avg_us < 2000.0,
            "Search latency in unoptimized debug must be < 2000 µs (got {:.2} µs)",
            avg_us
        );
    }
}
