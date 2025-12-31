#[cfg(feature = "download")]
use std::net::IpAddr;

#[cfg(feature = "download")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cache_path = "cache/ripe-data.txt";

    // 1) Update cache from real RIPE URL
    let bytes = eu_geoip::GeoIpDb::update_cache(cache_path)?;
    println!("Downloaded {bytes} bytes into {cache_path}");

    // 2) Load DB from cache (not embedded)
    let db = eu_geoip::GeoIpDb::from_ripe_delegated_file(cache_path)?;

    // 3) Try a lookup
    let ip: IpAddr = "88.198.0.1".parse()?; // commonly DE (Hetzner)
    if let Some(info) = db.lookup(ip) {
        println!("{ip} -> country={}, is_eu={}", info.country_code_str(), info.is_eu);
    } else {
        println!("{ip} not found");
    }

    Ok(())
}

#[cfg(not(feature = "download"))]
fn main() {
    eprintln!("This demo requires the `download` feature.");
    eprintln!("Run: cargo run --features download --bin ripe_update_demo");
}