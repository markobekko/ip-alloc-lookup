# ip-alloc-lookup

[![Crates.io](https://img.shields.io/crates/v/ip-alloc-lookup.svg)](https://crates.io/crates/ip-alloc-lookup)
[![Documentation](https://docs.rs/ip-alloc-lookup/badge.svg)](https://docs.rs/ip-alloc-lookup)

# Offline RIPE IP Allocation Lookup

A fast, offline IPv4 and IPv6 lookup library based on RIPE NCC delegated
allocation data. The crate maps IP addresses to ISO-3166 country codes and
includes a built-in European Union (EU) membership classification.

This is an **allocation-based lookup**, not a geolocation service.

---

## Key features

- **Very fast lookups** (nanosecond-scale per IP)
- **Offline-first**: no runtime network access required by default
- **IPv4 and IPv6 support**
- Based on **RIPE NCC delegated statistics**
- Built-in **EU membership classification**
- Thread-safe and read-only after initialization
- No mandatory external dependencies

---

## What this library represents

The data used by this library comes from **RIPE NCC delegated statistics**.
Each IP range is associated with:

- an ISO-3166 country code
- a boolean flag indicating whether that country is a member of the EU

The EU flag reflects **political membership**, derived from the country code.
It does **not** indicate where traffic is physically routed or processed.

### Important limitations

- Anycast, CDNs, and BGP routing may serve traffic from different locations
- Allocations do not guarantee physical data residency
- This library alone does not provide legal or regulatory compliance

---

## Installation

```toml
[dependencies]
ip-alloc-lookup = "0.1"
```

---

## Basic usage

### IPv4 lookup

```rust
use std::net::Ipv4Addr;
use ip_alloc_lookup::GeoIpDb;

let db = GeoIpDb::new();

let ip = Ipv4Addr::new(8, 8, 8, 8);
if let Some(info) = db.lookup_v4(ip) {
    println!("Country: {}", info.country_code_str());
    println!("EU member: {}", info.is_eu);
}
```

### IPv6 lookup

```rust
use std::net::Ipv6Addr;
use ip_alloc_lookup::GeoIpDb;

let db = GeoIpDb::new();

let ip = "2001:4860:4860::8888".parse::<Ipv6Addr>().unwrap();
if let Some(info) = db.lookup_v6(ip) {
    println!("Country: {}", info.country_code_str());
    println!("EU member: {}", info.is_eu);
}
```

---

## Unified lookup API

```rust
use std::net::IpAddr;
use ip_alloc_lookup::GeoIpDb;

let db = GeoIpDb::new();

let ip: IpAddr = "2a01:4f8::1".parse().unwrap();
if let Some(info) = db.lookup(ip) {
    println!("Country: {}", info.country_code_str());
    println!("EU member: {}", info.is_eu);
}
```

---

## Updating RIPE data at runtime (download feature)

By default, the crate ships with **embedded, pre-generated RIPE data**.
This guarantees deterministic behavior and zero network usage at runtime.

If you enable the optional `download` feature, the crate can:
- download the latest RIPE delegated statistics from the official RIPE URL
- cache them locally
- build a database from the cached file instead of the embedded data

### Enable the feature

```toml
[dependencies]
ip-alloc-lookup = { version = "0.1", features = ["download"] }
```

### Example: update cache, load database, and query

```rust
#[cfg(feature = "download")]
use std::net::IpAddr;

#[cfg(feature = "download")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cache_path = "cache/ripe-data.txt";

    // 1) Update cache from the official RIPE delegated URL
    let bytes = ip_alloc_lookup::GeoIpDb::update_cache(cache_path)?;
    println!("Downloaded {bytes} bytes into {cache_path}");

    // 2) Load database from cached file (not embedded data)
    let db = ip_alloc_lookup::GeoIpDb::from_ripe_delegated_file(cache_path)?;

    // 3) Perform a lookup
    let ip: IpAddr = "88.198.0.1".parse()?; // commonly DE (Hetzner)
    if let Some(info) = db.lookup(ip) {
        println!(
            "{ip} -> country={}, is_eu={}",
            info.country_code_str(),
            info.is_eu
        );
    } else {
        println!("{ip} not found");
    }

    Ok(())
}

#[cfg(not(feature = "download"))]
fn main() {
    eprintln!("This demo requires the `download` feature.");
    eprintln!("Run: cargo run --features download");
}
```

Once loaded, **all lookups are offline** and have the same performance
characteristics as the embedded database.

---

## EU membership classification

EU membership is determined by a built-in list of ISO-3166 country codes
corresponding to current EU member states.

This classification is:
- static
- deterministic
- derived from allocation metadata

It reflects **political membership**, not physical location or legal compliance.

---

## Performance

Lookups are implemented using binary search over sorted IP ranges and are
designed for hot paths.

Typical performance on modern x86_64 systems:

- **Single IPv4 lookup**: ~20 ns
- **Single IPv6 lookup**: ~18–20 ns
- **Batch lookups**: tens of millions of IPs per second
- **Zero allocations during lookup**

The database is immutable after construction and safe to share across threads.

All measurements were taken on a modern x86_64 system in release mode.
Results are representative and may vary depending on hardware, OS, and CPU
frequency scaling.

| Benchmark | Description | Median Result |
|---------|------------|---------------|
| `database_creation` | Build lookup tables | ~3.2 ms |
| `single_lookup_ipv4 (EU)` | Single IPv4 lookup (EU IP) | ~20 ns |
| `single_lookup_ipv4 (Non-EU)` | Single IPv4 lookup (Non-EU IP) | ~20–21 ns |
| `single_lookup_ipv6 (EU)` | Single IPv6 lookup (EU IP) | ~18–19 ns |
| `single_lookup_ipv6 (Non-EU)` | Single IPv6 lookup (Non-EU IP) | ~19–100 ns* |
| `unified_lookup IPv4 (EU)` | Unified `IpAddr` lookup | ~150–165 ns |
| `unified_lookup IPv4 (Non-EU)` | Unified `IpAddr` lookup | ~20–21 ns |
| `unified_lookup IPv6 (EU)` | Unified `IpAddr` lookup | ~19–20 ns |
| `unified_lookup IPv6 (Non-EU)` | Unified `IpAddr` lookup | ~20 ns |
| `is_eu_check IPv4` | EU membership check (IPv4) | ~28–60 ns |
| `is_eu_check IPv6` | EU membership check (IPv6) | ~20–21 ns |
| `batch_lookups_ipv4 (100)` | 100 IPv4 lookups | ~2.6 µs (~38 Melem/s) |
| `batch_lookups_ipv4 (1,000)` | 1,000 IPv4 lookups | ~28.5 µs (~35 Melem/s) |
| `batch_lookups_ipv4 (10,000)` | 10,000 IPv4 lookups | ~287 µs (~35 Melem/s) |
| `batch_lookups_ipv6 (100)` | 100 IPv6 lookups | ~2.0 µs (~50 Melem/s) |
| `batch_lookups_ipv6 (1,000)` | 1,000 IPv6 lookups | ~20.5 µs (~49 Melem/s) |
| `batch_lookups_ipv6 (10,000)` | 10,000 IPv6 lookups | ~211 µs (~47 Melem/s) |
| `worst_case_ipv4` | Worst-case IPv4 lookup | ~127 µs |
| `worst_case_ipv6` | Worst-case IPv6 lookup | ~110 µs |

\* IPv6 lookup latency depends on range density and position wit

### Benchmarking

The repository includes a comprehensive Criterion benchmark suite.

To run benchmarks locally:

```bash
cargo bench
```

Benchmarks cover:
- single IPv4 and IPv6 lookups
- unified `IpAddr` lookups
- EU membership checks
- batch lookups (100–10,000 IPs)
- worst-case and cache-hot scenarios
- multi-threaded lookups

Results show consistent nanosecond-scale performance for single lookups
and throughput in the tens of millions of lookups per second for batch
processing on commodity hardware.

---

## When should you use this crate?

✔ You need **fast, deterministic IP classification**  
✔ You run in **offline or restricted environments**  
✔ You care about **allocation metadata**, not precise geolocation  

## When you should not use it

✘ You need real-time physical location  
✘ You need routing-aware or anycast-aware answers  
✘ You need guaranteed legal or regulatory compliance  

---

## License

Licensed under either of

- Apache License, Version 2.0
- MIT license

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you shall be dual licensed as above, without any
additional terms or conditions.
