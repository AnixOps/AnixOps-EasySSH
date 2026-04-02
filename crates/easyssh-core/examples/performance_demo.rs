// 性能优化使用示例
// 运行: cargo run --example performance_demo --features lite

use easyssh_core::performance::*;
use easyssh_core::performance::{
    crypto_optimizer::CryptoOptimizer,
    memory_optimizer::MemoryOptimizer,
    search_optimizer::{FastStringMatcher, SearchOptimizer},
    startup_optimizer::StartupOptimizer,
};

fn main() {
    println!("EasySSH Lite 性能优化演示");
    println!("===========================\n");

    // 1. 启动优化演示
    println!("1. 启动优化");
    let startup = StartupOptimizer::new();
    startup.start().unwrap();
    println!("   - 启动序列已开始");

    // 2. 内存优化演示
    println!("\n2. 内存优化");
    let memory = MemoryOptimizer::new();
    {
        let buffer = memory.get_buffer().unwrap();
        println!("   - 获取缓冲区: {} bytes 容量", buffer.capacity());
    } // 自动归还
    {
        let string = memory.get_string().unwrap();
        println!("   - 获取字符串池: {} bytes 容量", string.capacity());
    } // 自动归还

    // 3. 搜索优化演示
    println!("\n3. 搜索优化");
    let search = SearchOptimizer::new();

    // 创建测试数据
    for i in 0..10 {
        let host = easyssh_core::db::HostRecord {
            id: format!("host-{}", i),
            name: format!("Production Server {}", i),
            host: format!("192.168.1.{}", i),
            port: 22,
            username: "admin".to_string(),
            auth_type: "key".to_string(),
            identity_file: None,
            identity_id: None,
            group_id: Some("prod".to_string()),
            notes: Some(format!("Server notes {}", i)),
            color: None,
            environment: Some("production".to_string()),
            region: Some("us-east".to_string()),
            purpose: Some("web".to_string()),
            status: "online".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        search.index_host(&host).unwrap();
    }

    let results = search.prefix_search("Prod", 5).unwrap();
    println!("   - 前缀搜索 'Prod': 找到 {} 个结果", results.len());

    // 4. 快速字符串匹配演示
    println!("\n4. 快速字符串匹配");
    let text = "Production Web Server - us-east-1";
    println!("   - 文本: '{}'", text);
    println!(
        "   - 包含 'web': {}",
        FastStringMatcher::contains(text, "web")
    );
    println!(
        "   - 前缀匹配 'Prod': {}",
        FastStringMatcher::starts_with(text, "Prod")
    );
    println!(
        "   - 模糊匹配 'pdc srv': {}",
        FastStringMatcher::fuzzy_match(text, "pdc srv")
    );
    println!(
        "   - 模糊评分 'prod': {:.2}",
        FastStringMatcher::fuzzy_score(text, "prod")
    );

    // 5. 加密优化演示
    println!("\n5. 加密优化");
    let crypto = CryptoOptimizer::new();
    let cache = crypto.key_cache();

    // 模拟缓存派生结果
    cache
        .cache_derivation("test_password", vec![1u8; 32], vec![2u8; 32])
        .unwrap();

    if cache.is_cached("test_password").unwrap() {
        println!("   - 密钥派生已缓存");
        let stats = cache.stats().unwrap();
        println!("   - 缓存条目数: {}", stats);
    }

    // 完成启动
    startup.complete().unwrap();

    // 性能报告
    println!("\n6. 性能报告");
    let report = startup.get_report().unwrap();
    println!("   - 总启动时间: {} ms", report.total_duration_ms);
    println!("   - 目标: < {} ms", report.target_ms);
    println!(
        "   - 达成目标: {}",
        if report.met_target() { "是" } else { "否" }
    );

    // 检查基准目标
    println!("\n7. 基准目标检查");
    println!("   - 冷启动目标: < {} ms", BenchmarkTargets::COLD_START_MS);
    println!(
        "   - 搜索响应目标: < {} ms",
        BenchmarkTargets::SEARCH_RESPONSE_MS
    );
    println!(
        "   - 内存使用目标: < {} MB",
        BenchmarkTargets::MEMORY_USAGE_MB
    );
    println!(
        "   - 数据库查询目标: < {} ms",
        BenchmarkTargets::DB_QUERY_MS
    );

    println!("\n演示完成！");
}
