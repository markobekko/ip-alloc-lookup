//! Build-time generation of embedded RIPE IP allocation tables.
//!
//! This build script parses a RIPE NCC delegated statistics file and converts it
//! into compact, sorted Rust data structures that are embedded into the final
//! binary.
//!
//! ## Why a build script?
//!
//! - Avoids parsing large text files at runtime
//! - Ensures deterministic, versioned data snapshots
//! - Enables zero-I/O startup
//!
//! ## Output
//!
//! The script generates a Rust source file in `OUT_DIR` containing:
//!
//! - A sorted IPv4 range table using `u32` addresses
//! - A sorted IPv6 range table using `u128` addresses
//!
//! These tables are later included by the library and used for binary search.
//!
//! ## IPv6 handling
//!
//! RIPE encodes IPv6 allocations using prefix lengths. During code generation,
//! these prefixes are expanded into inclusive `[start, end]` ranges to allow
//! direct numeric comparison at runtime.

use std::fs;
use std::io::Write;
use std::path::Path;

/// Build script: parses `ripe-data.txt` and emits `generated_data.rs` into `OUT_DIR`.
///
/// The generated file contains two sorted tables:
/// - `IPV4_RANGES: &[(u32, u32, &str)]`
/// - `IPV6_RANGES: &[(u128, u128, &str)]`
///
/// These tables are included by the library at compile time for fast, offline lookups.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=ripe-data.txt");

    // Read the RIPE data file
    let ripe_content = fs::read_to_string("ripe-data.txt")
        .expect("Failed to read ripe-data.txt - make sure it's in the project root");

    // Parse IPv4 and IPv6 separately
    let (v4_ranges, v6_ranges) = parse_ripe_data(&ripe_content);

    println!("cargo:warning=Parsed {} IPv4 ranges from RIPE data", v4_ranges.len());
    println!("cargo:warning=Parsed {} IPv6 ranges from RIPE data", v6_ranges.len());

    // Generate Rust code
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_data.rs");

    let mut file = fs::File::create(&dest_path).unwrap();

    // Write the header
    writeln!(file, "// Auto-generated from RIPE data at build time").unwrap();
    writeln!(file, "// DO NOT EDIT - changes will be overwritten").unwrap();
    writeln!(file, "").unwrap();

    // Write IPv4 ranges
    writeln!(
        file,
        "pub const IPV4_RANGES: &[(u32, u32, &str)] = &["
    )
    .unwrap();

    for (start, count, country) in &v4_ranges {
		if *count == 0 {
			continue; // shouldn't happen, but avoids underflow
		}
		let end = start.saturating_add(count.saturating_sub(1));
		writeln!(file, "    ({}, {}, \"{}\"),", start, end, country).unwrap();
	}

    writeln!(file, "];").unwrap();
    writeln!(file, "").unwrap();

    // Write IPv6 ranges
    if v6_ranges.is_empty() {
        // If no IPv6 data, create an empty array
        writeln!(file, "pub const IPV6_RANGES: &[(u128, u128, &str)] = &[];").unwrap();
    } else {
        writeln!(
            file,
            "pub const IPV6_RANGES: &[(u128, u128, &str)] = &["
        )
        .unwrap();

        for (start, end, country) in &v6_ranges {
			writeln!(file, "    ({}, {}, \"{}\"),", start, end, country).unwrap();
		}

        writeln!(file, "];").unwrap();
    }

    println!("cargo:warning=Generated data file with {} IPv4 ranges and {} IPv6 ranges", 
        v4_ranges.len(), v6_ranges.len());
}

/// Parse RIPE delegated stats content into sorted IPv4/IPv6 range lists for codegen.
///
/// For IPv4 lines, returns `(start_u32, count, country)`.
/// For IPv6 lines, RIPE’s “count” field is a prefix length; this converts it into an
/// inclusive end address and returns `(start_u128, end_u128, country)`.
///
/// The returned vectors are sorted by start address to enable binary search at runtime.
fn parse_ripe_data(content: &str) -> (Vec<(u32, u32, String)>, Vec<(u128, u128, String)>) {
    let mut v4_ranges = Vec::new();
    let mut v6_ranges = Vec::new();

    for line in content.lines() {
        // Skip comments and summary lines
        if line.starts_with('#') || line.starts_with('2') {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();

        if parts.len() < 7 {
            continue;
        }

        let country = parts[1].to_string();
        let ip_type = parts[2];
        let start_str = parts[3];
        let count_str = parts[4];

        if ip_type == "ipv4" {
            // Parse IPv4
            if let Ok(start_ip) = start_str.parse::<std::net::Ipv4Addr>() {
                if let Ok(count) = count_str.parse::<u32>() {
					if count == 0 { continue; }
                    let start_u32: u32 = start_ip.into();
                    v4_ranges.push((start_u32, count, country));
                }
            }
        } else if ip_type == "ipv6" {
            // Parse IPv6
            if let Ok(start_ip) = start_str.parse::<std::net::Ipv6Addr>() {
                if let Ok(prefix_len) = count_str.parse::<u32>() {
                    let start_u128: u128 = start_ip.into();
                    
                    // Calculate the number of addresses in this prefix
                    // For IPv6, the count field is actually the prefix length
                    // We need to calculate the end address
                    let host_bits = 128 - prefix_len;
                    let count = if host_bits >= 128 {
						u128::MAX
					} else {
						1u128 << host_bits
					};
					let end = start_u128.saturating_add(count).saturating_sub(1);
					v6_ranges.push((start_u128, end, country));
                }
            }
        }
    }

    // Sort ranges for binary search
    v4_ranges.sort_by_key(|r| r.0);
    v6_ranges.sort_by_key(|r| r.0);

    (v4_ranges, v6_ranges)
}