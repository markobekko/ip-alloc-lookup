//! # Offline RIPE-based IP Allocation Lookup
//!
//! This crate provides fast, offline lookups for IPv4 and IPv6 addresses based on
//! RIPE NCC delegated allocation data. It maps IP address ranges to ISO-3166
//! country codes and includes a built-in classification for European Union (EU)
//! membership.
//!
//! ## What this crate does
//!
//! - Performs **purely offline** IP lookups using pre-generated range tables.
//! - Supports **IPv4 and IPv6** with logarithmic-time lookups.
//! - Uses **RIPE delegated statistics**, not active geolocation or probing.
//! - Associates each IP range with a **country code** and an **EU membership flag**.
//!
//! ## What this crate does NOT do
//!
//! - It does **not** determine the physical location of hosts or users.
//! - It does **not** track BGP routing, anycast behavior, or traffic paths.
//! - It does **not** provide legal or regulatory compliance guarantees.
//!
//! The country and EU information reflect the **RIR allocation or assignment
//! metadata** published by RIPE NCC. In real-world networks, traffic may be served
//! from different locations due to CDNs, anycast, tunneling, or routing policies.
//!
//! ## Design goals
//!
//! - Predictable performance (no syscalls or I/O on lookup).
//! - Deterministic results (static data, no runtime mutation).
//! - Minimal memory overhead and no mandatory external dependencies.
//!
//! ## Typical use cases
//!
//! - High-throughput IP classification in hot paths (firewalls, proxies, logging).
//! - Allocation-based policy checks (e.g. EU vs non-EU).
//! - Offline or restricted environments where external services are unavailable.
//!
//! This crate should be understood as an **IP allocation lookup**, not a
//! geolocation service.
mod database;

// Re-export public API
pub use database::{GeoIpDb, GeoInfo, DbStats};

// We keep the parser public for users who want to work with raw RIPE data
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, PartialEq)]
pub struct IpRange {
    pub start_v4: Option<Ipv4Addr>,
    pub start_v6: Option<Ipv6Addr>,
    pub count: u128,
    pub country: String,
}

/// Parses RIPE delegated stats format for both IPv4 and IPv6
/// This is exposed for advanced users who want to process RIPE data themselves
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