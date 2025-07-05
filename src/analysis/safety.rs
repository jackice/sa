use crate::analysis::calculate_metaspace;
use crate::args::Args;
use colored::Colorize;

/// 安全性分析结果
///
/// # 字段说明
/// - `heap_safety`: 堆内存安全系数 (0-1), 1表示完全安全
/// - `direct_mem_safety`: 直接内存安全系数 (0-1)
/// - `risk_level`: 整体风险等级描述
/// - `scenarios`: 模拟的不同负载场景
/// - `recommendations`: 优化建议列表
pub struct SafetyAnalysis {
    pub heap_safety: f64,                      // 堆内存安全系数 (0-1)
    pub direct_mem_safety: f64,                // 直接内存安全系数 (0-1)
    pub risk_level: String,                    // 整体风险等级
    pub scenarios: Vec<Scenario>,              // 模拟场景
    pub recommendations: Vec<String>,          // 优化建议
    pub theoretical_limits: TheoreticalLimits, // 理论极限评估
}

/// 理论极限评估(基于6-12个月稳定运行)
pub struct TheoreticalLimits {
    pub max_connections: usize,     // 在稳定运行条件下的最大连接数
    pub max_throughput: f64,        // 可持续吞吐量(MB/s)
    pub estimated_uptime: String,   // 预估稳定运行时长分类
    pub limiting_factor: String,    // 主要瓶颈资源
    pub burst_capacity: usize,      // 突发流量承载能力
    pub resource_breakdown: String, // 各资源利用率分析
}

pub struct Scenario {
    pub name: String,
    pub connections: usize,
    pub file_size: f64,
    pub heap_usage: f64,       // GB
    pub direct_mem_usage: f64, // GB
    pub status: String,        // 安全/警告/危险
}

/// 动态计算每个连接的直接内存需求
fn calculate_direct_mem_per_conn(file_size: f64) -> (f64, f64) {
    // 读缓冲区大小 (动态调整)
    let read_buffer = if file_size <= 10.0 {
        128.0 // 128KB for small files
    } else if file_size <= 100.0 {
        512.0 // 512KB for medium files
    } else {
        // For large files, use 1MB buffer but allow chunked processing
        // with memory mapping optimization
        (1024.0_f64).min(file_size * 0.01) // 1MB or 1% of file size, whichever is smaller
    };

    // 写缓冲区大小 (通常比读缓冲区大)
    let write_buffer = read_buffer * 1.5;

    // 额外开销 (SSL/TLS, headers etc)
    let overhead = 100.0; // 100KB fixed overhead

    (
        read_buffer / 1024.0 / 1024.0,               // convert to GB
        (write_buffer + overhead) / 1024.0 / 1024.0, // convert to GB
    )
}

pub fn calculate_safety(args: &Args, direct_mem_gb: f64, heap_mem_gb: f64) -> SafetyAnalysis {
    const HEAP_PER_CONN: f64 = 384.0 / 1024.0 / 1024.0; // 384KB -> GB (含对象开销)

    // 计算正常场景内存使用 (动态调整缓冲区大小)
    let (read_buffer_per_conn, write_buffer_per_conn) =
        calculate_direct_mem_per_conn(args.avg_file_size);
    let normal_direct_usage =
        args.expected_connections as f64 * (read_buffer_per_conn + write_buffer_per_conn);

    // 如果是大文件(>100MB)且使用内存映射，可以减少直接内存需求
    let mem_map_reduction = if args.avg_file_size > 100.0 && args.enable_memory_mapping {
        0.5 // 内存映射可减少50%直接内存需求
    } else {
        1.0
    };
    let normal_direct_usage = normal_direct_usage * mem_map_reduction;
    let normal_heap_usage = args.expected_connections as f64 * HEAP_PER_CONN;

    // 计算突发场景内存使用
    let burst_connections = (args.expected_connections as f64 * args.burst_factor) as usize;
    let (burst_read, burst_write) = calculate_direct_mem_per_conn(args.avg_file_size);
    let burst_direct_usage = burst_connections as f64 * (burst_read + burst_write);
    let burst_heap_usage = burst_connections as f64 * HEAP_PER_CONN;

    // 计算安全系数 (0-1)，保留15%给JVM Native内存
    const JVM_NATIVE_RATIO: f64 = 0.15;
    let available_heap = heap_mem_gb * (1.0 - JVM_NATIVE_RATIO);
    let available_direct = direct_mem_gb * (1.0 - JVM_NATIVE_RATIO);

    // 使用更保守的安全阈值(0.7)
    let heap_safety = 1.0 - (normal_heap_usage / (available_heap * 0.7)).min(1.0);
    let direct_mem_safety = 1.0 - (normal_direct_usage / (available_direct * 0.7)).min(1.0);

    // 改进的风险等级评估
    let risk_level = match (heap_safety, direct_mem_safety) {
        (h, d) if h > 0.4 && d > 0.4 => "低风险".to_string(),
        (h, d) if h > 0.2 || d > 0.2 => "中风险".to_string(),
        _ => "高风险".to_string(),
    };

    // 创建模拟场景
    let mut scenarios = Vec::new();

    // 场景1: 长期运行(24小时)
    scenarios.push(Scenario {
        name: "长期运行(24h)".to_string(),
        connections: args.expected_connections,
        file_size: args.avg_file_size,
        heap_usage: normal_heap_usage * 1.5, // 假设长期运行堆增长50%
        direct_mem_usage: normal_direct_usage * 1.2, // 直接内存增长20%
        status: status_label(
            normal_heap_usage * 1.5,
            heap_mem_gb,
            normal_direct_usage * 1.2,
            direct_mem_gb,
        ),
    });

    // 场景2: 正常负载
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

    // 增强长期运行评估和建议
    let heap_growth_rate = normal_heap_usage * 0.05; // 假设每小时堆增长5%
    let oom_hours = ((heap_mem_gb * 0.9 - normal_heap_usage) / heap_growth_rate).max(0.0);

    recommendations.push(format!(
        "- 内存泄漏评估: 当前配置可能在{oom_hours:.1}小时后发生OOM"
    ));
    recommendations.push("- 添加内存监控: 实时监控堆/直接内存的增长率".to_string());
    recommendations.push("- 启用GC日志分析: 建议使用Prometheus+Grafana监控".to_string());
    recommendations.push("- 启用堆转储: 设置-XX:+HeapDumpOnOutOfMemoryError".to_string());

    if oom_hours < 24.0 {
        recommendations.push("❗ 紧急: 内存泄漏风险高，需要立即优化".red().to_string());
    }

    // 计算理论极限
    let theoretical_limits = calculate_theoretical_limits(
        args,
        direct_mem_gb,
        heap_mem_gb,
        normal_direct_usage,
        normal_heap_usage,
    );

    SafetyAnalysis {
        heap_safety,
        direct_mem_safety,
        risk_level,
        scenarios,
        recommendations,
        theoretical_limits,
    }
}

/// 计算理论极限值(基于JVM推荐配置和6-12个月稳定运行目标)
fn calculate_theoretical_limits(
    args: &Args,
    direct_mem_gb: f64,
    heap_mem_gb: f64,
    normal_direct_usage: f64,
    normal_heap_usage: f64,
) -> TheoreticalLimits {
    // 基于JVM推荐配置的资源消耗模型
    const HEAP_PER_CONN: f64 = 384.0 / 1024.0 / 1024.0; // 384KB/连接(含对象开销)
    const METASPACE_PER_CONN: f64 = 64.0 / 1024.0; // 64KB/连接
    const CPU_PER_CONN: f64 = 0.0005; // 每个连接占用的CPU资源(核)
    const NET_PER_CONN: f64 = 0.2; // 每个连接平均带宽(Mbps)
    const DISK_IO_PER_CONN: f64 = 0.15; // 每个连接IOPS需求

    // 长期稳定性系数
    const STABILITY_FACTOR: f64 = 0.6; // 只使用60%资源保证长期稳定
    const SAFE_MEM_USAGE: f64 = 0.7; // 更保守的内存使用阈值

    // 1. 计算各维度极限(考虑突发流量)
    let burst_connections = (args.expected_connections as f64 * args.burst_factor) as usize;

    // 动态计算每个连接的直接内存需求
    let (read_buffer, write_buffer) = calculate_direct_mem_per_conn(args.avg_file_size);
    let direct_mem_per_conn = read_buffer + write_buffer;

    // 内存限制(基于动态计算)
    let max_by_direct = if args.enable_memory_mapping && args.avg_file_size > 100.0 {
        // 内存映射优化可支持更多连接
        ((direct_mem_gb * SAFE_MEM_USAGE) / (direct_mem_per_conn * 0.7) * STABILITY_FACTOR) as usize
    } else {
        ((direct_mem_gb * SAFE_MEM_USAGE) / direct_mem_per_conn * STABILITY_FACTOR) as usize
    };
    let max_by_heap = ((heap_mem_gb * SAFE_MEM_USAGE) / HEAP_PER_CONN * STABILITY_FACTOR) as usize;

    // 元空间限制(基于动态计算结果)
    let metaspace_size_mb = calculate_metaspace(args) as f64;
    let max_by_metaspace = ((metaspace_size_mb * 1024.0 * 1024.0)
        / (METASPACE_PER_CONN * args.expected_connections as f64)
        * STABILITY_FACTOR) as usize;

    // CPU限制(考虑上下文切换开销)
    let max_by_cpu = ((args.cpu_cores as f64 / CPU_PER_CONN) * STABILITY_FACTOR) as usize;

    // 网络限制
    let max_by_net = ((args.net_gbps * 1000.0 / NET_PER_CONN) * STABILITY_FACTOR) as usize;

    // 磁盘IO限制(基于SSD性能模型)
    let (disk_iops, _disk_suggestion) = match args.disk_type.as_str() {
        "nvme" => (500_000.0, None),
        "sata_ssd" => {
            if args.expected_connections > 50_000 {
                (100_000.0, Some("考虑升级到NVMe SSD"))
            } else {
                (100_000.0, None)
            }
        }
        _ => (200.0, Some("必须升级到SSD")), // HDD
    };
    let max_by_disk = ((disk_iops / DISK_IO_PER_CONN) * STABILITY_FACTOR) as usize;

    // 综合极限(取最小值，考虑JVM各维度限制)
    let max_connections = max_by_direct
        .min(max_by_heap)
        .min(max_by_metaspace)
        .min(max_by_cpu)
        .min(max_by_net)
        .min(max_by_disk)
        .min(burst_connections); // 必须满足突发需求

    // 2. 计算可持续吞吐量(考虑长期负载均衡)
    let sustainable_throughput = (args.cpu_cores as f64 * STABILITY_FACTOR) / 0.15; // 0.15秒/MB处理时间

    // 3. 长期运行评估(6-12个月)
    let uptime_category = if max_connections >= burst_connections * 2 {
        "12个月+ (弹性充足)"
    } else if max_connections >= burst_connections {
        "6-12个月 (满足需求)"
    } else {
        "<6个月 (需扩容)"
    };

    // 4. 确定瓶颈资源
    let limiting_factor = if max_connections == max_by_direct {
        "直接内存"
    } else if max_connections == max_by_heap {
        "堆内存"
    } else if max_connections == max_by_cpu {
        "CPU资源"
    } else if max_connections == max_by_net {
        "网络带宽"
    } else if max_connections == max_by_disk {
        "磁盘IO"
    } else {
        "突发流量需求"
    };

    // 5. 生成资源利用率分析(包含JVM维度)
    let resource_breakdown = format!(
        "    * JVM内存: {:.0}% (堆), {:.0}% (直接), {:.0}% (元空间)\n    * CPU: {:.0}%\n    * 网络: {:.0}%\n    * 磁盘IO: {:.0}%",
        (normal_heap_usage / (heap_mem_gb * SAFE_MEM_USAGE) * 100.0).min(100.0),
        (normal_direct_usage / (direct_mem_gb * SAFE_MEM_USAGE) * 100.0).min(100.0),
        (args.expected_connections as f64 * METASPACE_PER_CONN * 100.0
            / (metaspace_size_mb * 1024.0 * 1024.0))
            .min(100.0),
        (args.expected_connections as f64 / max_by_cpu as f64 * 100.0).min(100.0),
        (args.expected_connections as f64 / max_by_net as f64 * 100.0).min(100.0),
        (args.expected_connections as f64 / max_by_disk as f64 * 100.0).min(100.0)
    );

    TheoreticalLimits {
        max_connections,
        max_throughput: sustainable_throughput,
        estimated_uptime: uptime_category.to_string(),
        limiting_factor: limiting_factor.to_string(),
        burst_capacity: (max_connections as f64 / STABILITY_FACTOR) as usize,
        resource_breakdown,
    }
}

fn status_label(heap_usage: f64, heap_max: f64, direct_usage: f64, direct_max: f64) -> String {
    // 考虑JVM自身开销(15%)和长期运行余量(15%)
    let effective_heap_max = heap_max * 0.7;
    let effective_direct_max = direct_max * 0.7;

    let heap_ratio = heap_usage / effective_heap_max;
    let direct_ratio = direct_usage / effective_direct_max;

    match (heap_ratio, direct_ratio) {
        (h, d) if h < 0.6 && d < 0.6 => "✅ 安全".green().to_string(),
        (h, d) if h < 0.8 || d < 0.8 => "⚠️ 警告".yellow().to_string(),
        _ => "🔥 危险".red().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Args;

    #[test]
    fn test_calculate_safety() {
        let args = Args {
            total_ram: 16.0,
            cpu_cores: 8,
            net_gbps: 1.0,
            disk_type: "sata_ssd".to_string(),
            expected_connections: 1000,
            burst_factor: 2.0,
            avg_file_size: 5.0,
            enable_memory_guard: true,
            enable_memory_mapping: false,
            complexity: "medium".to_string(),
            generate_markdown: false,
        };
        let safety = calculate_safety(&args, 4.0, 12.0);
        assert!(safety.heap_safety > 0.0, "Heap safety should be positive");
        assert!(
            safety.direct_mem_safety > 0.0,
            "Direct memory safety should be positive"
        );
        assert!(!safety.scenarios.is_empty(), "Should generate scenarios");
        assert!(
            !safety.recommendations.is_empty(),
            "Should generate recommendations"
        );
    }
}
