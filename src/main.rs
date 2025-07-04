use clap::Parser;
use colored::*;
use std::collections::HashMap;

/// 文件上传下载系统性能与安全性分析工具
#[derive(Parser, Debug)]
#[clap(version = "3.2", author = "System Safety Analyst")]
struct Args {
    /// 服务器总内存(GB)
    #[clap(short, long, default_value = "32")]
    total_ram: f64,

    /// CPU核心数
    #[clap(short = 'c', long, default_value = "16")]
    cpu_cores: usize,

    /// 网络带宽(Gbps)
    #[clap(short, long, default_value = "1")]
    net_gbps: f64,

    /// 磁盘类型 [sata_hdd, sata_ssd, nvme]
    #[clap(short, long, default_value = "sata_ssd")]
    disk_type: String,

    /// 平均文件大小(MB)
    #[clap(short, long, default_value = "10")]
    avg_file_size: f64,

    /// 预期最大并发连接数
    #[clap(short = 'n', long, default_value = "1000")]
    expected_connections: usize,

    /// 最大突发流量倍数
    #[clap(short = 'b', long, default_value = "3")]
    burst_factor: f64,

    /// 是否启用内存防护 [true, false]
    #[clap(short = 'p', long, default_value = "true")]
    enable_memory_guard: bool,

    /// 应用复杂度级别 [low, medium, high]
    #[clap(short = 'l', long, default_value = "medium")]
    complexity: String,
}

struct SafetyAnalysis {
    heap_safety: f64,             // 堆内存安全系数 (0-1)
    direct_mem_safety: f64,       // 直接内存安全系数 (0-1)
    risk_level: String,           // 整体风险等级
    scenarios: Vec<Scenario>,     // 模拟场景
    recommendations: Vec<String>, // 优化建议
}

struct Scenario {
    name: String,
    connections: usize,
    file_size: f64,
    heap_usage: f64,       // GB
    direct_mem_usage: f64, // GB
    status: String,        // 安全/警告/危险
}

fn main() {
    let args = Args::parse();

    // 磁盘速度映射
    let disk_speeds: HashMap<&str, (f64, f64)> = [
        ("sata_hdd", (120.0, 100.0)),
        ("sata_ssd", (300.0, 250.0)),
        ("nvme", (1500.0, 1200.0)),
    ]
    .iter()
    .cloned()
    .collect();

    // 验证磁盘类型
    if !disk_speeds.contains_key::<str>(&args.disk_type.as_str()) {
        eprintln!("错误: 不支持的磁盘类型. 可用选项: sata_hdd, sata_ssd, nvme");
        std::process::exit(1);
    }

    // 获取磁盘速度
    let (disk_read_speed, disk_write_speed) = disk_speeds[args.disk_type.as_str()];

    // 1. 计算内存分配
    let direct_mem_gb = (args.total_ram * 0.08).max(1.0);
    let heap_mem_gb = (args.total_ram * 0.35).max(4.0);

    // 2. 动态计算元空间大小
    let metaspace_size_mb = calculate_metaspace(&args);

    // 3. 计算安全系数
    let safety = calculate_safety(&args, direct_mem_gb, heap_mem_gb);

    // 4. 打印系统配置
    print_configuration(
        &args,
        direct_mem_gb,
        heap_mem_gb,
        metaspace_size_mb,
        disk_read_speed,
        disk_write_speed,
    );

    // 5. 打印安全性报告
    print_safety_report(&safety);

    // 6. 打印场景模拟
    print_scenarios(&safety);

    // 7. 打印JVM配置建议
    print_jvm_recommendations(
        &args,
        direct_mem_gb,
        heap_mem_gb,
        metaspace_size_mb,
        &safety,
    );
}

/// 动态计算元空间大小
fn calculate_metaspace(args: &Args) -> i32 {
    // 基础值 (MB)
    let mut base = 512.0;

    // 根据应用复杂度调整
    match args.complexity.as_str() {
        "low" => base *= 0.8,  // 简单应用
        "high" => base *= 1.5, // 复杂应用（大量类加载）
        _ => {}                // medium 保持不变
    }

    // 根据并发连接数调整 (每1000连接增加50MB)
    let connection_factor = (args.expected_connections as f64 / 1000.0).floor() * 50.0;

    // 根据文件大小调整 (大文件处理需要更多元数据)
    let file_size_factor = (args.avg_file_size / 50.0).min(4.0) * 50.0;

    // 应用安全系数
    let total = (base + connection_factor + file_size_factor) * 1.25;

    // 限制在合理范围
    total.max(256.0).min(2048.0).ceil() as i32
}

fn calculate_safety(args: &Args, direct_mem_gb: f64, heap_mem_gb: f64) -> SafetyAnalysis {
    // 常量定义
    const BUFFER_PER_CONN: f64 = 256.0 / 1024.0 / 1024.0; // 256KB -> GB
    const OVERHEAD_PER_CONN: f64 = 50.0 / 1024.0 / 1024.0; // 50KB -> GB
    const HEAP_PER_CONN: f64 = 256.0 / 1024.0 / 1024.0; // 256KB -> GB

    // 计算正常场景内存使用
    let normal_direct_usage =
        args.expected_connections as f64 * (BUFFER_PER_CONN + OVERHEAD_PER_CONN);
    let normal_heap_usage = args.expected_connections as f64 * HEAP_PER_CONN;

    // 计算突发场景内存使用
    let burst_connections = (args.expected_connections as f64 * args.burst_factor) as usize;
    let burst_direct_usage = burst_connections as f64 * (BUFFER_PER_CONN + OVERHEAD_PER_CONN);
    let burst_heap_usage = burst_connections as f64 * HEAP_PER_CONN;

    // 计算安全系数 (0-1)
    let heap_safety = 1.0 - (normal_heap_usage / (heap_mem_gb * 0.8)).min(1.0);
    let direct_mem_safety = 1.0 - (normal_direct_usage / (direct_mem_gb * 0.8)).min(1.0);

    // 确定整体风险等级
    let risk_level = if heap_safety > 0.3 && direct_mem_safety > 0.3 {
        "低风险".to_string()
    } else if heap_safety > 0.15 && direct_mem_safety > 0.15 {
        "中风险".to_string()
    } else {
        "高风险".to_string()
    };

    // 创建模拟场景
    let mut scenarios = Vec::new();

    // 场景1: 正常负载
    scenarios.push(Scenario {
        name: "正常负载".to_string(),
        connections: args.expected_connections,
        file_size: args.avg_file_size,
        heap_usage: normal_heap_usage,
        direct_mem_usage: normal_direct_usage,
        status: status_label(
            normal_heap_usage,
            heap_mem_gb,
            normal_direct_usage,
            direct_mem_gb,
        ),
    });

    // 场景2: 突发流量
    scenarios.push(Scenario {
        name: format!("突发流量 ({}x)", args.burst_factor),
        connections: burst_connections,
        file_size: args.avg_file_size,
        heap_usage: burst_heap_usage,
        direct_mem_usage: burst_direct_usage,
        status: status_label(
            burst_heap_usage,
            heap_mem_gb,
            burst_direct_usage,
            direct_mem_gb,
        ),
    });

    // 场景3: 大文件处理
    scenarios.push(Scenario {
        name: "大文件处理".to_string(),
        connections: (args.expected_connections as f64 * 0.5) as usize,
        file_size: args.avg_file_size * 5.0,
        heap_usage: normal_heap_usage * 0.5,
        direct_mem_usage: normal_direct_usage * 0.5,
        status: status_label(
            normal_heap_usage * 0.5,
            heap_mem_gb,
            normal_direct_usage * 0.5,
            direct_mem_gb,
        ),
    });

    // 场景4: 小文件高并发
    scenarios.push(Scenario {
        name: "小文件高并发".to_string(),
        connections: args.expected_connections * 3,
        file_size: args.avg_file_size / 10.0,
        heap_usage: normal_heap_usage * 1.5,
        direct_mem_usage: normal_direct_usage * 1.5,
        status: status_label(
            normal_heap_usage * 1.5,
            heap_mem_gb,
            normal_direct_usage * 1.5,
            direct_mem_gb,
        ),
    });

    // 生成优化建议
    let mut recommendations = Vec::new();

    if direct_mem_safety < 0.3 {
        recommendations.push(format!(
            "- 增加直接内存: {:.1}GB -> {:.1}GB",
            direct_mem_gb,
            direct_mem_gb * 1.3
        ));
    }

    if heap_safety < 0.3 {
        recommendations.push(format!(
            "- 增加堆内存: {:.1}GB -> {:.1}GB",
            heap_mem_gb,
            heap_mem_gb * 1.2
        ));
    }

    if args.enable_memory_guard {
        recommendations.push("- 启用内存防护系统: 当内存使用>85%时自动限流".to_string());
    }

    if args.avg_file_size > 50.0 {
        recommendations.push("- 优化大文件处理: 使用分块上传和内存映射文件".to_string());
    }

    SafetyAnalysis {
        heap_safety,
        direct_mem_safety,
        risk_level,
        scenarios,
        recommendations,
    }
}

fn status_label(heap_usage: f64, heap_max: f64, direct_usage: f64, direct_max: f64) -> String {
    let heap_ratio = heap_usage / heap_max;
    let direct_ratio = direct_usage / direct_max;

    if heap_ratio < 0.7 && direct_ratio < 0.7 {
        "✅ 安全".green().to_string()
    } else if heap_ratio < 0.85 && direct_ratio < 0.85 {
        "⚠️ 警告".yellow().to_string()
    } else {
        "🔥 危险".red().to_string()
    }
}

fn print_configuration(
    args: &Args,
    direct_mem_gb: f64,
    heap_mem_gb: f64,
    metaspace_size_mb: i32,
    disk_read_speed: f64,
    disk_write_speed: f64,
) {
    println!(
        "\n{}{}",
        "▬".cyan().bold().reversed(),
        " 系统配置 ".cyan().bold().reversed()
    );
    println!("{}", "▬".cyan().bold().repeated(50));

    let config_table = vec![
        ("服务器内存", format!("{:.1} GB", args.total_ram)),
        ("CPU核心数", format!("{}", args.cpu_cores)),
        ("网络带宽", format!("{:.1} Gbps", args.net_gbps)),
        (
            "磁盘类型",
            format!(
                "{} (读: {:.0} MB/s, 写: {:.0} MB/s)",
                args.disk_type, disk_read_speed, disk_write_speed
            ),
        ),
        ("平均文件大小", format!("{:.1} MB", args.avg_file_size)),
        ("预期并发连接", format!("{}", args.expected_connections)),
        ("突发流量倍数", format!("{}x", args.burst_factor)),
        ("内存防护", format!("{}", args.enable_memory_guard)),
        ("应用复杂度", format!("{}", args.complexity)),
    ];

    for (label, value) in config_table {
        println!("  {:>20}: {}", label.cyan(), value);
    }

    println!("\n  {:>20}: {:.1} GB", "推荐堆内存".cyan(), heap_mem_gb);
    println!("  {:>20}: {:.1} GB", "推荐直接内存".cyan(), direct_mem_gb);
    println!(
        "  {:>20}: {} MB (动态计算)",
        "元空间".cyan(),
        metaspace_size_mb
    );
}

fn print_safety_report(safety: &SafetyAnalysis) {
    println!(
        "\n{}{}",
        "▬".yellow().bold().reversed(),
        " 安全性分析报告 ".yellow().bold().reversed()
    );
    println!("{}", "▬".yellow().bold().repeated(50));

    // 风险等级
    let risk_color = match safety.risk_level.as_str() {
        "低风险" => "green",
        "中风险" => "yellow",
        _ => "red",
    };

    println!(
        "  {:>20}: {}",
        "整体风险等级".cyan(),
        safety.risk_level.color(risk_color).bold()
    );

    // 安全系数图表
    println!("\n  {}{}", "内存安全系数".cyan(), " (0-1, 越高越安全):");

    print_safety_bar("堆内存安全", safety.heap_safety);
    print_safety_bar("直接内存安全", safety.direct_mem_safety);

    // 防护建议
    if !safety.recommendations.is_empty() {
        println!("\n  {}{}", "优化建议".cyan(), ":");
        for rec in &safety.recommendations {
            println!("    - {}", rec);
        }
    }
}

fn print_safety_bar(label: &str, value: f64) {
    let width = 30;
    let fill = (value * width as f64) as usize;
    let empty = width - fill;

    let bar = format!(
        "[{}{}] {:.0}%",
        "■".green().repeated(fill),
        " ".repeated(empty),
        value * 100.0
    );

    println!("  {:>18}: {}", label.cyan(), bar);
}

fn print_scenarios(safety: &SafetyAnalysis) {
    println!(
        "\n{}{}",
        "▬".blue().bold().reversed(),
        " 场景模拟分析 ".blue().bold().reversed()
    );
    println!("{}", "▬".blue().bold().repeated(50));

    println!(
        "  {:<18} {:<12} {:<12} {:<12} {:<12} {:<10}",
        "场景".cyan(),
        "连接数".cyan(),
        "文件大小".cyan(),
        "堆内存".cyan(),
        "直接内存".cyan(),
        "状态".cyan()
    );

    for scenario in &safety.scenarios {
        println!(
            "  {:<18} {:<12} {:<12.1} {:<12.2} {:<12.2} {}",
            scenario.name,
            scenario.connections,
            scenario.file_size,
            scenario.heap_usage,
            scenario.direct_mem_usage,
            scenario.status
        );
    }

    // 解释状态标识
    println!("\n  {}: <70% 内存使用", "✅ 安全".green());
    println!("  {}: 70-85% 内存使用", "⚠️ 警告".yellow());
    println!("  {}: >85% 内存使用", "🔥 危险".red());
}

fn print_jvm_recommendations(
    args: &Args,
    direct_mem_gb: f64,
    heap_mem_gb: f64,
    metaspace_size_mb: i32,
    safety: &SafetyAnalysis,
) {
    println!(
        "\n{}{}",
        "▬".green().bold().reversed(),
        " JVM配置建议 ".green().bold().reversed()
    );
    println!("{}", "▬".green().bold().repeated(50));

    // 基础配置
    println!("{}", "  # 基础配置".bold());
    println!("  -Xms{0}g -Xmx{0}g", heap_mem_gb as i32);
    println!("  -XX:MaxDirectMemorySize={}g", direct_mem_gb as i32);
    println!(
        "  -XX:MaxMetaspaceSize={}m  # 动态计算值",
        metaspace_size_mb
    );
    println!("  -XX:ReservedCodeCacheSize=256m");

    // 内存防护增强
    println!("\n{}", "  # 内存防护增强".bold());
    println!("  -XX:+UseG1GC");
    println!("  -XX:MaxGCPauseMillis=200");
    println!(
        "  -XX:ParallelGCThreads={}",
        (args.cpu_cores as f64 * 0.5).ceil() as i32
    );
    println!(
        "  -XX:ConcGCThreads={}",
        (args.cpu_cores as f64 * 0.25).ceil() as i32
    );

    if safety.direct_mem_safety < 0.4 {
        println!("  -Djdk.nio.maxCachedBufferSize=131072  # 降低缓存阈值至128KB");
    } else {
        println!("  -Djdk.nio.maxCachedBufferSize=262144  # 256KB缓存阈值");
    }

    if args.enable_memory_guard {
        println!("  -Dapp.memory.guard.enabled=true");
        println!(
            "  -Dapp.memory.guard.direct.threshold={:.1}g",
            direct_mem_gb * 0.85
        );
        println!(
            "  -Dapp.memory.guard.heap.threshold={:.1}g",
            heap_mem_gb * 0.8
        );
    }

    // 元空间优化（针对高复杂度应用）
    if args.complexity == "high" {
        println!("\n{}", "  # 元空间优化（高复杂度应用）".bold());
        println!("  -XX:+UseCompressedClassPointers");
        println!(
            "  -XX:CompressedClassSpaceSize={}m",
            (metaspace_size_mb as f32 * 0.4).max(256.0) as i32
        );
        println!("  -XX:+UnlockExperimentalVMOptions");
        println!("  -XX:+UseZGC  # 可选：针对大堆内存使用ZGC");
    }

    // 监控配置
    println!("\n{}", "  # 监控与诊断".bold());
    println!("  -XX:NativeMemoryTracking=detail");
    println!("  -XX:+PrintGCDetails -XX:+PrintGCDateStamps");
    println!("  -XX:+HeapDumpOnOutOfMemoryError");
    println!("  -XX:HeapDumpPath=/var/log/jvm_dumps");

    // 大文件优化
    if args.avg_file_size > 50.0 {
        println!("\n{}", "  # 大文件优化".bold());
        println!("  -Djdk.nio.enableFastFileTransfer=true");
        println!("  -Dapp.file.maxChunkSize=2097152  # 2MB分块");
        println!("  -Dapp.file.useDirectIO=true");
    }

    println!("\n{}", "  # 启动命令示例".bold());
    println!("  java \\");
    println!("    -Xms{0}g -Xmx{0}g \\", heap_mem_gb as i32);
    println!("    -XX:MaxDirectMemorySize={}g \\", direct_mem_gb as i32);
    println!("    -XX:MaxMetaspaceSize={}m \\", metaspace_size_mb);
    println!("    -XX:ReservedCodeCacheSize=256m \\");
    println!("    -jar your-application.jar");
}

// 扩展trait用于重复字符串
trait Repeated {
    fn repeated(&self, times: usize) -> String;
}

impl Repeated for &str {
    fn repeated(&self, times: usize) -> String {
        self.repeat(times)
    }
}

impl Repeated for ColoredString {
    fn repeated(&self, times: usize) -> String {
        self.to_string().repeat(times)
    }
}
