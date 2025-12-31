use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::{fs, io, path::Path};

#[cfg(feature = "download")]
pub const RIPE_EXTENDED_LATEST_URL: &str =
    "https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest";

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GeoInfo {
    pub country_code: [u8; 2],
    pub is_eu: bool,
    pub region: u8,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    EuropeanUnion = 1,
    EuropeNonEu   = 2,
    EasternEurope = 3,
    Turkey        = 4,
    MiddleEast    = 5,
    NorthAfrica   = 6,
    CentralAsia   = 7,
    GulfStates    = 8,
    Other         = 255,
}

impl Region {
    pub fn as_str(self) -> &'static str {
        match self {
            Region::EuropeanUnion => "European Union",
            Region::EuropeNonEu   => "Europe (non-EU)",
            Region::EasternEurope => "Eastern Europe",
            Region::Turkey        => "Turkey",
            Region::MiddleEast    => "Middle East",
            Region::NorthAfrica   => "North Africa",
            Region::CentralAsia   => "Central Asia",
            Region::GulfStates    => "Gulf States",
            Region::Other         => "Other",
        }
    }
}

// Convert a 2-letter country code like "DE" into [b'D', b'E'].
fn cc2(country: &str) -> [u8; 2] {
    let b = country.as_bytes();
    // RIPE data should always be 2-letter country codes; if not, fall back.
    if b.len() >= 2 { [b[0], b[1]] } else { *b"??" }
}

// For display/testing convenience.
impl GeoInfo {
    pub fn country_code_str(&self) -> &str {
        // Always valid for ASCII 2-letter codes; fallback if somehow invalid.
        std::str::from_utf8(&self.country_code).unwrap_or("??")
    }

    pub fn region_enum(&self) -> Region {
        match self.region {
            1 => Region::EuropeanUnion,
            2 => Region::EuropeNonEu,
            3 => Region::EasternEurope,
            4 => Region::Turkey,
            5 => Region::MiddleEast,
            6 => Region::NorthAfrica,
            7 => Region::CentralAsia,
            8 => Region::GulfStates,
            _ => Region::Other,
        }
    }
}


pub struct GeoIpDb {
    v4_ranges: Vec<(u32, u32, GeoInfo)>,
    v6_ranges: Vec<(u128, u128, GeoInfo)>,
}

// EU member states (27 countries as of 2025)
const EU_COUNTRIES: &[&str] = &[
    "AT", "BE", "BG", "HR", "CY", "CZ", "DK", "EE", "FI", "FR",
    "DE", "GR", "HU", "IE", "IT", "LV", "LT", "LU", "MT", "NL",
    "PL", "PT", "RO", "SK", "SI", "ES", "SE",
];

// Include the generated data from build.rs
include!(concat!(env!("OUT_DIR"), "/generated_data.rs"));

impl GeoIpDb {
    /// Create a new database with embedded RIPE data
    pub fn new() -> Self {
        let mut v4_ranges = Vec::with_capacity(IPV4_RANGES.len());
        let mut v6_ranges = Vec::with_capacity(IPV6_RANGES.len());

        // Process IPv4 ranges
        for &(start, end, country) in IPV4_RANGES {
            let is_eu = EU_COUNTRIES.contains(&country);
            let region = determine_region(country);

            let geo_info = GeoInfo {
				country_code: cc2(country),
				is_eu,
				region: region as u8,
			};

            v4_ranges.push((start, end, geo_info));
        }

        // Process IPv6 ranges
        for &(start, end, country) in IPV6_RANGES {
            let is_eu = EU_COUNTRIES.contains(&country);
            let region = determine_region(country);

            let geo_info = GeoInfo {
				country_code: cc2(country),
				is_eu,
				region: region as u8,
			};

            v6_ranges.push((start, end, geo_info));
        }

        // Data should already be sorted from build.rs, but let's be safe
        //v4_ranges.sort_by_key(|r| r.0);
        //v6_ranges.sort_by_key(|r| r.0);

        GeoIpDb { v4_ranges, v6_ranges }
    }
	
	/// Build a DB from RIPE delegated stats *content* (runtime).
    pub fn from_ripe_delegated_str(content: &str) -> Self {
        let parsed = crate::parse_ripe_delegated(content);

        let mut v4_ranges: Vec<(u32, u32, GeoInfo)> = Vec::new();
        let mut v6_ranges: Vec<(u128, u128, GeoInfo)> = Vec::new();

        for r in parsed {
            let is_eu = EU_COUNTRIES.contains(&r.country.as_str());
            let region = determine_region(&r.country);

            let geo = GeoInfo {
                country_code: cc2(&r.country),
                is_eu,
                region: region as u8,
            };

            if let Some(v4) = r.start_v4 {
                let start: u32 = v4.into();
                let end = start.saturating_add((r.count as u32).saturating_sub(1));
                v4_ranges.push((start, end, geo));
            } else if let Some(v6) = r.start_v6 {
                let start: u128 = v6.into();
                let end = start.saturating_add(r.count.saturating_sub(1));
                v6_ranges.push((start, end, geo));
            }
        }

        v4_ranges.sort_by_key(|r| r.0);
        v6_ranges.sort_by_key(|r| r.0);

        GeoIpDb { v4_ranges, v6_ranges }
    }

    /// Load RIPE delegated stats data from a file at runtime.
    pub fn from_ripe_delegated_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(Self::from_ripe_delegated_str(&content))
    }

    /// Try to load from a cache file; if missing/unreadable, fall back to embedded data.
    pub fn from_cache_or_embedded<P: AsRef<Path>>(cache_path: P) -> Self {
        match Self::from_ripe_delegated_file(cache_path) {
            Ok(db) => db,
            Err(_) => Self::new(),
        }
    }

    /// Look up an IPv4 address
	#[inline]
    pub fn lookup_v4(&self, ip: Ipv4Addr) -> Option<&GeoInfo> {
		let ip_u32: u32 = ip.into();
		
		match self.v4_ranges.binary_search_by_key(&ip_u32, |&(start, _, _)| start) {
			Ok(idx) => Some(&self.v4_ranges[idx].2),
			Err(idx) => {
				if idx > 0 {
					let (start, end, geo) = &self.v4_ranges[idx - 1];
					if ip_u32 >= *start && ip_u32 <= *end {
						return Some(geo);
					}
				}
				None
			}
		}
	}

    /// Look up an IPv6 address
	#[inline]
	pub fn lookup_v6(&self, ip: Ipv6Addr) -> Option<&GeoInfo> {
		let ip_u128: u128 = ip.into();
		let ranges = &self.v6_ranges;

		if ranges.is_empty() {
			return None;
		}

		// upper_bound: first index where start > ip
		let mut lo: usize = 0;
		let mut hi: usize = ranges.len();
		while lo < hi {
			let mid = lo + (hi - lo) / 2;
			if ip_u128 < ranges[mid].0 {
				hi = mid;
			} else {
				lo = mid + 1;
			}
		}

		if lo == 0 {
			return None;
		}

		let (start, end, geo) = &ranges[lo - 1];
		if ip_u128 >= *start && ip_u128 <= *end {
			Some(geo)
		} else {
			None
		}
	}

    /// Look up any IP address (IPv4 or IPv6)
    pub fn lookup(&self, ip: IpAddr) -> Option<&GeoInfo> {
        match ip {
            IpAddr::V4(v4) => self.lookup_v4(v4),
            IpAddr::V6(v6) => self.lookup_v6(v6),
        }
    }

    /// Convenience method: check if IP is in EU
	#[inline]
    pub fn is_eu(&self, ip: IpAddr) -> bool {
        self.lookup(ip).map(|info| info.is_eu).unwrap_or(false)
    }

    /// Get statistics about the database
    pub fn stats(&self) -> DbStats {
        let total_v4_ranges = self.v4_ranges.len();
        let total_v6_ranges = self.v6_ranges.len();
        let eu_v4_ranges = self.v4_ranges.iter().filter(|(_, _, info)| info.is_eu).count();
        let eu_v6_ranges = self.v6_ranges.iter().filter(|(_, _, info)| info.is_eu).count();

        DbStats {
            total_v4_ranges,
            total_v6_ranges,
            eu_v4_ranges,
            eu_v6_ranges,
            non_eu_v4_ranges: total_v4_ranges - eu_v4_ranges,
            non_eu_v6_ranges: total_v6_ranges - eu_v6_ranges,
        }
    }
}

#[cfg(feature = "download")]
impl GeoIpDb {
    /// Download RIPE delegated data from `url` and atomically replace `cache_path`.
    ///
    /// Returns the number of bytes written.
    pub fn update_cache_from_url<P: AsRef<Path>>(cache_path: P, url: &str) -> io::Result<u64> {
        let cache_path = cache_path.as_ref();

        // Ensure parent dir exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Download
        let resp = reqwest::blocking::get(url)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
            .error_for_status()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let bytes = resp
            .bytes()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Write to a temp file next to the destination (so rename is atomic on most OSes)
        let tmp_path = cache_path.with_extension("tmp");
        {
            let mut f = fs::File::create(&tmp_path)?;
            use std::io::Write;
            f.write_all(&bytes)?;
            f.sync_all()?;
        }

        // Replace existing cache atomically-ish
        if cache_path.exists() {
            // On Windows rename can fail if target exists, so remove first.
            let _ = fs::remove_file(cache_path);
        }
        fs::rename(&tmp_path, cache_path)?;

        Ok(bytes.len() as u64)
    }

    /// Convenience: update from RIPE "extended latest".
    pub fn update_cache<P: AsRef<Path>>(cache_path: P) -> io::Result<u64> {
        Self::update_cache_from_url(cache_path, RIPE_EXTENDED_LATEST_URL)
    }
}

impl Default for GeoIpDb {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct DbStats {
    pub total_v4_ranges: usize,
    pub total_v6_ranges: usize,
    pub eu_v4_ranges: usize,
    pub eu_v6_ranges: usize,
    pub non_eu_v4_ranges: usize,
    pub non_eu_v6_ranges: usize,
}

fn determine_region(country_code: &str) -> Region {
    if EU_COUNTRIES.contains(&country_code) {
        Region::EuropeanUnion
    } else {
        match country_code {
            "GB" | "NO" | "CH" | "IS" | "LI" => Region::EuropeNonEu,
            "RU" | "UA" | "BY" | "MD" => Region::EasternEurope,
            "TR" => Region::Turkey,
            "IL" | "PS" => Region::MiddleEast,
            "EG" | "TN" | "MA" | "DZ" => Region::NorthAfrica,
            "KZ" | "UZ" | "TM" | "KG" | "TJ" => Region::CentralAsia,
            "AE" | "SA" | "QA" | "KW" | "BH" | "OM" => Region::GulfStates,
            _ => Region::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_db() {
        let db = GeoIpDb::new();

        let stats = db.stats();
        println!("\n📊 Embedded Database Stats:");
        println!("  IPv4 ranges: {} (EU: {}, non-EU: {})", 
            stats.total_v4_ranges, stats.eu_v4_ranges, stats.non_eu_v4_ranges);
        println!("  IPv6 ranges: {} (EU: {}, non-EU: {})", 
            stats.total_v6_ranges, stats.eu_v6_ranges, stats.non_eu_v6_ranges);

        assert!(stats.total_v4_ranges > 0, "Should have IPv4 ranges");
    }

    #[test]
    fn test_lookup_german_ipv4() {
        let db = GeoIpDb::new();
        let ip: Ipv4Addr = "46.4.0.1".parse().unwrap();

        let info = db.lookup_v4(ip).expect("German IP should be found");
        assert_eq!(info.country_code_str(), "DE");
        assert!(info.is_eu);
    }

    #[test]
    fn test_lookup_german_ipv6() {
        let db = GeoIpDb::new();
        // Example German IPv6 address (2a00::/12 is typically EU)
        let ip: Ipv6Addr = "2a01:4f8::1".parse().unwrap();

        if let Some(info) = db.lookup_v6(ip) {
            println!("Found IPv6: {} in {}", ip, info.country_code_str());
            // Just verify we can look it up, actual country depends on data
        }
    }

    #[test]
    fn test_lookup_any_ip() {
        let db = GeoIpDb::new();
        
        // Test with IPv4
        let ipv4: IpAddr = "46.4.0.1".parse().unwrap();
        if let Some(info) = db.lookup(ipv4) {
            assert_eq!(info.country_code_str(), "DE");
        }

        // Test with IPv6
        let ipv6: IpAddr = "2a01:4f8::1".parse().unwrap();
        let _ = db.lookup(ipv6);
    }

    #[test]
    fn test_is_eu_method() {
        let db = GeoIpDb::new();

        // Test IPv4
        let ipv4: IpAddr = "46.4.0.1".parse().unwrap();
        if db.lookup(ipv4).is_some() {
            assert!(db.is_eu(ipv4));
        }
    }
	
	#[cfg(feature = "download")]
	fn serve_once(body: &'static str) -> String {
		use std::io::{Read, Write};
		use std::net::TcpListener;

		let listener = TcpListener::bind("127.0.0.1:0").unwrap();
		let addr = listener.local_addr().unwrap();

		std::thread::spawn(move || {
			let (mut stream, _) = listener.accept().unwrap();

			// read request (ignore contents)
			let mut buf = [0u8; 1024];
			let _ = stream.read(&mut buf);

			let resp = format!(
				"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
				body.as_bytes().len(),
				body
			);
			let _ = stream.write_all(resp.as_bytes());
			let _ = stream.flush();
		});

		format!("http://{}", addr)
	}
	
	#[test]
	#[cfg(feature = "download")]
	fn test_update_cache_and_load() {
		use std::net::IpAddr;

		// Minimal delegated content:
		// - one IPv4 block: 46.4.0.0/24 (256 addrs)
		// - one IPv6 block: 2a01:4f8::/32
		let delegated = "\
	# comment
	2|ripencc|20250101|0000|summary|whatever
	ripencc|DE|ipv4|46.4.0.0|256|20250101|allocated
	ripencc|DE|ipv6|2a01:4f8::|32|20250101|allocated
	";

		let url = serve_once(delegated);

		let dir = tempfile::tempdir().unwrap();
		let cache_path = dir.path().join("ripe-cache.txt");

		let bytes = GeoIpDb::update_cache_from_url(&cache_path, &url).unwrap();
		assert!(bytes > 0);
		assert!(cache_path.exists());

		let db = GeoIpDb::from_ripe_delegated_file(&cache_path).unwrap();

		let ip: IpAddr = "46.4.0.1".parse().unwrap();
		let info = db.lookup(ip).expect("should find 46.4.0.1");
		assert_eq!(info.country_code_str(), "DE");
		assert!(info.is_eu);
	}
	
	#[test]
	#[cfg(feature = "download")]
	fn test_update_cache_replaces_existing_file() {
		let old = "\
	ripencc|FR|ipv4|46.4.0.0|256|20250101|allocated
	";
		let new = "\
	ripencc|DE|ipv4|46.4.0.0|256|20250101|allocated
	";

		let url = serve_once(new);

		let dir = tempfile::tempdir().unwrap();
		let cache_path = dir.path().join("ripe-cache.txt");

		std::fs::write(&cache_path, old).unwrap();

		GeoIpDb::update_cache_from_url(&cache_path, &url).unwrap();

		let db = GeoIpDb::from_ripe_delegated_file(&cache_path).unwrap();
		let info = db.lookup("46.4.0.1".parse().unwrap()).unwrap();
		assert_eq!(info.country_code_str(), "DE");
	}
	
	#[test]
	#[ignore]
	#[cfg(feature = "download")]
	fn smoke_test_real_ripe_download_and_lookup() {
		let cache = std::path::PathBuf::from("/tmp/ripe-cache.txt");

		// Download real RIPE data
		let bytes = GeoIpDb::update_cache(&cache).unwrap();
		assert!(bytes > 1_000_000, "too small, download probably failed");

		// Load from cache
		let db = GeoIpDb::from_ripe_delegated_file(&cache).unwrap();

		// Known Hetzner range is commonly DE
		let ip: std::net::IpAddr = "88.198.0.1".parse().unwrap();
		let info = db.lookup(ip).unwrap();
		println!("88.198.0.1 -> {}", info.country_code_str());
	}
}