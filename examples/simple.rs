use ip_alloc_lookup::GeoIpDb;

fn main() {
    println!("Simple ip-alloc-lookup Usage Example\n");
    
    // Create database (loads embedded data)
    let db = GeoIpDb::new();
    
    // Check some IPv4 addresses
    let test_ipv4 = vec![
        "46.4.0.1",       // German
        "80.80.80.80",    // German
        "91.198.174.192", // Dutch (Wikipedia)
        "5.3.0.1",        // Russian
        "151.101.1.69",   // Fastly CDN
    ];
    
    // Check some IPv6 addresses
    let test_ipv6 = vec![
        "2a01:4f8::1",           // Hetzner (German)
        "2001:67c:2e8:22::c100:68b", // Wikipedia (Dutch)
        "2a00:1450:4001::68",    // Google (Ireland)
        "2a03:2880:f003::68",    // Facebook (Ireland)
        "2001:41d0::1",          // OVH (France)
    ];
    
    println!("Testing IPv4 addresses:\n");
    for ip_str in test_ipv4 {
        let ip = ip_str.parse().unwrap();
        match db.lookup(ip) {
            Some(info) => {
                let symbol = if info.is_eu { "EU" } else { "Not EU" };
                println!(
					"{} {} -> {} ({})",
					symbol,
					ip_str,
					info.country_code_str(),
					info.region_enum().as_str()
				);
            }
            None => {
                println!(" {} -> Not in RIPE database", ip_str);
            }
        }
    }
    
    println!("\nTesting IPv6 addresses:\n");
    for ip_str in test_ipv6 {
        let ip = ip_str.parse().unwrap();
        match db.lookup(ip) {
            Some(info) => {
                let symbol = if info.is_eu { "EU" } else { "Not EU" };
                println!(
					"{} {} -> {} ({})",
					symbol,
					ip_str,
					info.country_code_str(),
					info.region_enum().as_str()
				);
            }
            None => {
                println!(" {} -> Not in RIPE database", ip_str);
            }
        }
    }
    
    // Quick EU check for both IPv4 and IPv6
    println!("\nQuick EU checks:");
    println!("  Is 46.4.0.1 (IPv4) in EU? {}", db.is_eu("46.4.0.1".parse().unwrap()));
    println!("  Is 5.3.0.1 (IPv4) in EU? {}", db.is_eu("5.3.0.1".parse().unwrap()));
    println!("  Is 2a01:4f8::1 (IPv6) in EU? {}", db.is_eu("2a01:4f8::1".parse().unwrap()));
    
    // Database stats
    let stats = db.stats();
    println!("\nDatabase info:");
    println!("  Total IPv4 ranges: {}", stats.total_v4_ranges);
    println!("  Total IPv6 ranges: {}", stats.total_v6_ranges);
    
    if stats.total_v4_ranges > 0 {
        println!("  EU IPv4 ranges: {} ({:.1}%)",
            stats.eu_v4_ranges,
            (stats.eu_v4_ranges as f64 / stats.total_v4_ranges as f64) * 100.0
        );
    }
    
    if stats.total_v6_ranges > 0 {
        println!("  EU IPv6 ranges: {} ({:.1}%)",
            stats.eu_v6_ranges,
            (stats.eu_v6_ranges as f64 / stats.total_v6_ranges as f64) * 100.0
        );
    } else {
        println!("  No IPv6 data in database yet");
    }
}