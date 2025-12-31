use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use ip_alloc_lookup::GeoIpDb;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use rand::{rngs::StdRng, Rng, SeedableRng};

fn generate_random_ipv4(count: usize, seed: u64) -> Vec<Ipv4Addr> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..count)
        .map(|_| Ipv4Addr::from(rng.r#gen::<u32>()))
        .collect()
}

fn generate_random_ipv6(count: usize, seed: u64) -> Vec<Ipv6Addr> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..count)
        .map(|_| Ipv6Addr::from(rng.r#gen::<u128>()))
        .collect()
}

fn generate_mixed_ips(count: usize, seed: u64) -> Vec<IpAddr> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..count)
        .map(|_| {
            if rng.gen_bool(0.5) {
                IpAddr::V4(Ipv4Addr::from(rng.r#gen::<u32>()))
            } else {
                IpAddr::V6(Ipv6Addr::from(rng.r#gen::<u128>()))
            }
        })
        .collect()
}

fn benchmark_db_creation(c: &mut Criterion) {
    c.bench_function("database_creation", |b| {
        b.iter(|| {
            let db = GeoIpDb::new();
            black_box(db);
        })
    });
}

fn benchmark_single_lookup_ipv4(c: &mut Criterion) {
    let db = GeoIpDb::new();
    
    let test_ips = vec![
        ("EU IP (Germany)", "46.4.0.1"),
        ("EU IP (Netherlands)", "145.220.0.1"),
        ("Non-EU IP (Russia)", "5.3.0.1"),
        ("Non-EU IP (Turkey)", "151.101.1.69"),
    ];
    
    let mut group = c.benchmark_group("single_lookup_ipv4");
    
    for (name, ip_str) in test_ips {
        let ip: Ipv4Addr = ip_str.parse().unwrap();
        
        group.bench_with_input(BenchmarkId::new("lookup_v4", name), &ip, |b, ip| {
            b.iter(|| {
                let result = db.lookup_v4(*ip);
                black_box(result);
            });
        });
    }
    
    group.finish();
}

fn benchmark_single_lookup_ipv6(c: &mut Criterion) {
    let db = GeoIpDb::new();
    
    let test_ips = vec![
        ("Hetzner DE", "2a01:4f8::1"),
        ("Google IE", "2a00:1450:4001::68"),
        ("OVH FR", "2001:41d0::1"),
        ("Cloudflare", "2606:4700::1"),
    ];
    
    let mut group = c.benchmark_group("single_lookup_ipv6");
    
    for (name, ip_str) in test_ips {
        let ip: Ipv6Addr = ip_str.parse().unwrap();
        
        group.bench_with_input(BenchmarkId::new("lookup_v6", name), &ip, |b, ip| {
            b.iter(|| {
                let result = db.lookup_v6(*ip);
                black_box(result);
            });
        });
    }
    
    group.finish();
}

fn benchmark_unified_lookup(c: &mut Criterion) {
    let db = GeoIpDb::new();
    
    let test_ips: Vec<(&str, IpAddr)> = vec![
        ("IPv4 EU", "46.4.0.1".parse().unwrap()),
        ("IPv4 Non-EU", "5.3.0.1".parse().unwrap()),
        ("IPv6 EU", "2a01:4f8::1".parse().unwrap()),
        ("IPv6 Non-EU", "2606:4700::1".parse().unwrap()),
    ];
    
    let mut group = c.benchmark_group("unified_lookup");
    
    for (name, ip) in test_ips {
        group.bench_with_input(BenchmarkId::new("lookup", name), &ip, |b, ip| {
            b.iter(|| {
                let result = db.lookup(*ip);
                black_box(result);
            });
        });
    }
    
    group.finish();
}

fn benchmark_is_eu_method(c: &mut Criterion) {
    let db = GeoIpDb::new();
    
    let mut group = c.benchmark_group("is_eu_check");
    
    let ipv4: IpAddr = "46.4.0.1".parse().unwrap();
    group.bench_function("ipv4", |b| {
        b.iter(|| {
            let result = db.is_eu(ipv4);
            black_box(result);
        })
    });
    
    let ipv6: IpAddr = "2a01:4f8::1".parse().unwrap();
    group.bench_function("ipv6", |b| {
        b.iter(|| {
            let result = db.is_eu(ipv6);
            black_box(result);
        })
    });
    
    group.finish();
}

fn benchmark_batch_lookups_ipv4(c: &mut Criterion) {
    let db = GeoIpDb::new();
    let ips = generate_random_ipv4(10_000, 0xDEADBEEF);
    
    let mut group = c.benchmark_group("batch_lookups_ipv4");
    
    for &batch_size in &[100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &size| {
                b.iter(|| {
                    for ip in ips.iter().take(size) {
                        let result = db.lookup_v4(*ip);
                        black_box(result);
                    }
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_batch_lookups_ipv6(c: &mut Criterion) {
    let db = GeoIpDb::new();
    let ips = generate_random_ipv6(10_000, 0xDEADBEEF);
    
    let mut group = c.benchmark_group("batch_lookups_ipv6");
    
    for &batch_size in &[100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &size| {
                b.iter(|| {
                    for ip in ips.iter().take(size) {
                        let result = db.lookup_v6(*ip);
                        black_box(result);
                    }
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_batch_lookups_mixed(c: &mut Criterion) {
    let db = GeoIpDb::new();
    let ips = generate_mixed_ips(10_000, 0xDEADBEEF);
    
    let mut group = c.benchmark_group("batch_lookups_mixed");
    
    for &batch_size in &[100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &size| {
                b.iter(|| {
                    for ip in ips.iter().take(size) {
                        let result = db.lookup(*ip);
                        black_box(result);
                    }
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_worst_case_ipv4(c: &mut Criterion) {
    let db = GeoIpDb::new();
    let ips = generate_random_ipv4(5_000, 0xBADCAFE);
    
    c.bench_function("worst_case_ipv4", |b| {
        b.iter(|| {
            for ip in &ips {
                let result = db.lookup_v4(*ip);
                black_box(result);
            }
        })
    });
}

fn benchmark_worst_case_ipv6(c: &mut Criterion) {
    let db = GeoIpDb::new();
    let ips = generate_random_ipv6(5_000, 0xBADCAFE);
    
    c.bench_function("worst_case_ipv6", |b| {
        b.iter(|| {
            for ip in &ips {
                let result = db.lookup_v6(*ip);
                black_box(result);
            }
        })
    });
}

fn benchmark_cache_performance(c: &mut Criterion) {
    let db = GeoIpDb::new();
    // Simulate cache-friendly access pattern (same IPs repeatedly)
    let hot_ipv4 = vec![
        "46.4.0.1".parse::<Ipv4Addr>().unwrap(),
        "80.80.80.80".parse::<Ipv4Addr>().unwrap(),
        "5.3.0.1".parse::<Ipv4Addr>().unwrap(),
    ];
    
    let hot_ipv6 = vec![
        "2a01:4f8::1".parse::<Ipv6Addr>().unwrap(),
        "2a00:1450:4001::68".parse::<Ipv6Addr>().unwrap(),
    ];
    
    let mut group = c.benchmark_group("cache_performance");
    
    group.bench_function("hot_ipv4_repeated", |b| {
        b.iter(|| {
            for _ in 0..100 {
                for ip in &hot_ipv4 {
                    let result = db.lookup_v4(*ip);
                    black_box(result);
                }
            }
        })
    });
    
    group.bench_function("hot_ipv6_repeated", |b| {
        b.iter(|| {
            for _ in 0..100 {
                for ip in &hot_ipv6 {
                    let result = db.lookup_v6(*ip);
                    black_box(result);
                }
            }
        })
    });
    
    group.finish();
}

fn benchmark_stats(c: &mut Criterion) {
    let db = GeoIpDb::new();
    
    c.bench_function("database_stats", |b| {
        b.iter(|| {
            let stats = db.stats();
            black_box(stats);
        })
    });
}

criterion_group!(
    benches,
    benchmark_db_creation,
    benchmark_single_lookup_ipv4,
    benchmark_single_lookup_ipv6,
    benchmark_unified_lookup,
    benchmark_is_eu_method,
    benchmark_batch_lookups_ipv4,
    benchmark_batch_lookups_ipv6,
    benchmark_batch_lookups_mixed,
    benchmark_worst_case_ipv4,
    benchmark_worst_case_ipv6,
    benchmark_cache_performance,
    benchmark_stats
);

criterion_main!(benches);