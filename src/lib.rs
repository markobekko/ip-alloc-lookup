//! Offline IP-to-country and region classification based on RIPE NCC data.
//!
//! This crate provides a lightweight, allocation-based alternative to
//! MaxMind-style GeoIP databases. Instead of city-level precision, it focuses on:
//!
//! - Country code (ISO-3166 alpha-2)
//! - Coarse regional grouping (EU / non-EU, etc.)
//! - Fully offline, deterministic lookups
//!
//! ## Data source
//!
//! The database is built from RIPE NCC “delegated statistics” files, which list
//! IPv4 and IPv6 address allocations by country. These files are:
//!
//! - Public
//! - Regularly updated
//! - Easy to parse and cache
//!
//! By default, a preprocessed snapshot is embedded at compile time for
//! zero-I/O runtime lookups.
//!
//! ## Design goals
//!
//! - No runtime network access required
//! - Minimal memory usage
//! - Fast lookups via binary search
//! - Simple API suitable for policy decisions (e.g. GDPR / EU checks)
//!
//! ## Limitations
//!
//! This crate does **not** attempt to provide:
//!
//! - City or ISP precision
//! - User location inference
//! - Dynamic routing awareness
//!
//! It reflects allocation data, not actual physical location.

mod database;

// Re-export public API
pub use database::{GeoIpDb, GeoInfo, DbStats};

// We keep the parser public for users who want to work with raw RIPE data
use std::net::{Ipv4Addr, Ipv6Addr};

/// A single allocation block parsed from a RIPE delegated statistics file.
///
/// For IPv4 blocks, `start_v4` is `Some` and `start_v6` is `None`.
/// For IPv6 blocks, `start_v6` is `Some` and `start_v4` is `None`.
///
/// `count` is the number of addresses in the block. For IPv6 lines, RIPE uses a
/// prefix length in the “count” field; this parser converts that prefix length
/// into an address count (`2^(128-prefix_len)`).
#[derive(Debug, Clone, PartialEq)]
pub struct IpRange {
    pub start_v4: Option<Ipv4Addr>,
    pub start_v6: Option<Ipv6Addr>,
    pub count: u128,
    pub country: String,
}

/// Parse RIPE NCC “delegated-*” statistics content into allocation ranges.
///
/// This parser is intentionally simple:
/// - Ignores comment lines (`#...`) and summary/header lines starting with `2`.
/// - Accepts only `ipv4` and `ipv6` records.
/// - Keeps the two-letter country code exactly as present in the file.
///
/// For IPv4 records, `count` is the number of addresses.
/// For IPv6 records, RIPE encodes the *prefix length* in the “count” field; this
/// function converts it to an address count.
///
/// # Examples
/// ```
/// use offline_ripe_geoip::parse_ripe_delegated;
///
/// let data = "ripencc|DE|ipv4|46.4.0.0|256|20250101|allocated\n";
/// let ranges = parse_ripe_delegated(data);
/// assert_eq!(ranges.len(), 1);
/// assert_eq!(ranges[0].country, "DE");
/// ```
///
/// # Notes
/// This does not validate that the returned ranges are non-overlapping or sorted.
pub fn parse_ripe_delegated(content: &str) -> Vec<IpRange> {
    content
        .lines()
        .filter(|line| {
            !line.starts_with('#')
                && !line.starts_with('2')
                && (line.contains("ipv4") || line.contains("ipv6"))
        })
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('|').collect();

            if parts.len() < 7 {
                return None;
            }

            let ip_type = parts[2];
            let country = parts[1].to_string();

            if ip_type == "ipv4" {
                Some(IpRange {
                    start_v4: parts[3].parse().ok(),
                    start_v6: None,
                    count: parts[4].parse::<u32>().ok()? as u128,
                    country,
                })
            } else if ip_type == "ipv6" {
                // For IPv6, the count field is actually the prefix length
                let prefix_len: u32 = parts[4].parse().ok()?;
                let host_bits = 128 - prefix_len;
                let count = if host_bits >= 128 {
                    u128::MAX
                } else {
                    1u128 << host_bits
                };

                Some(IpRange {
                    start_v4: None,
                    start_v6: parts[3].parse().ok(),
                    count,
                    country,
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn test_basic_usage() {
        // This is what users will actually do
        let db = GeoIpDb::new();

        println!("\n Testing basic library usage:");

        // Test a German IPv4
        let ipv4: IpAddr = "46.4.0.1".parse().unwrap();
        if let Some(info) = db.lookup(ipv4) {
            println!("  46.4.0.1 -> {} (EU: {})", info.country_code_str(), info.is_eu);
            assert!(info.is_eu);
        }

        // Test convenience method
        let is_eu = db.is_eu("46.4.0.1".parse().unwrap());
        println!("  is_eu(46.4.0.1) = {}", is_eu);

        // Show stats
        let stats = db.stats();
        println!("  Database: {} IPv4 ranges ({} EU, {} non-EU)",
            stats.total_v4_ranges, stats.eu_v4_ranges, stats.non_eu_v4_ranges);
        println!("            {} IPv6 ranges ({} EU, {} non-EU)",
            stats.total_v6_ranges, stats.eu_v6_ranges, stats.non_eu_v6_ranges);
    }

    #[test]
    fn test_ipv6_lookup() {
        let db = GeoIpDb::new();

        // Try to look up a common European IPv6 address
        let ipv6: IpAddr = "2a01:4f8::1".parse().unwrap();
        
        if let Some(info) = db.lookup(ipv6) {
            println!("  2a01:4f8::1 -> {} (EU: {})", info.country_code_str(), info.is_eu);
        } else {
            println!("  2a01:4f8::1 not found in database");
        }
    }
}