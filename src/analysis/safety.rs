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

pub fn calculate_safety(args: &Args, direct_mem_gb: f64, heap_mem_gb: f64) -> SafetyAnalysis {
    // 常量定义
    // 文件传输场景需要更大的直接内存缓冲
    const READ_BUFFER_PER_CONN: f64 = 512.0 / 1024.0 / 1024.0; // 512KB -> GB
    const WRITE_BUFFER_PER_CONN: f64 = 1.0 / 1024.0; // 1MB -> GB 
    const OVERHEAD_PER_CONN: f64 = 100.0 / 1024.0 / 1024.0; // 100KB -> GB
    // Removed unused constant
    const HEAP_PER_CONN: f64 = 256.0 / 1024.0 / 1024.0; // 256KB -> GB

    // 计算正常场景内存使用
    let normal_direct_usage = args.expected_connections as f64
        * (READ_BUFFER_PER_CONN + WRITE_BUFFER_PER_CONN + OVERHEAD_PER_CONN);
    let normal_heap_usage = args.expected_connections as f64 * HEAP_PER_CONN;

    // 计算突发场景内存使用
    let burst_connections = (args.expected_connections as f64 * args.burst_factor) as usize;
    let burst_direct_usage = burst_connections as f64
        * (READ_BUFFER_PER_CONN + WRITE_BUFFER_PER_CONN + OVERHEAD_PER_CONN);
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

    // 长期运行防护建议
    recommendations.push("- 定期重启服务: 建议每24小时滚动重启一次".to_string());
    recommendations.push("- 添加内存监控: 监控堆/直接内存的长期增长趋势".to_string());
    recommendations.push("- 启用GC日志分析: 使用工具定期分析GC日志".to_string());

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
    const DIRECT_MEM_PER_CONN: f64 = 512.0 / 1024.0 / 1024.0; // 512KB/连接(含安全缓冲)
    const HEAP_PER_CONN: f64 = 384.0 / 1024.0 / 1024.0; // 384KB/连接(含对象开销)
    const METASPACE_PER_CONN: f64 = 64.0 / 1024.0; // 64KB/连接
    const CPU_PER_CONN: f64 = 0.0005; // 每个连接占用的CPU资源(核)
    const NET_PER_CONN: f64 = 0.2; // 每个连接平均带宽(Mbps)
    const DISK_IO_PER_CONN: f64 = 0.15; // 每个连接IOPS需求

    // 稳定性系数(6-12个月稳定运行)
    const STABILITY_FACTOR: f64 = 0.7; // 只使用70%资源保证长期稳定
    const SAFE_MEM_USAGE: f64 = 0.75; // 内存安全使用阈值

    // 1. 计算各维度极限(考虑突发流量)
    let burst_connections = (args.expected_connections as f64 * args.burst_factor) as usize;

    // 内存限制(基于JVM推荐配置)
    let max_by_direct =
        ((direct_mem_gb * SAFE_MEM_USAGE) / DIRECT_MEM_PER_CONN * STABILITY_FACTOR) as usize;
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
        },
        _ => (200.0, Some("必须升级到SSD")) // HDD
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Args;

    #[test]
    fn test_calculate_safety() {
        let args = Args {
            expected_connections: 1000,
            burst_factor: 3.0,
            avg_file_size: 10.0,
            ..Default::default()
        };
        let safety = calculate_safety(&args, 2.0, 8.0);
        assert!(safety.heap_safety > 0.0);
        assert!(safety.direct_mem_safety > 0.0);
    }
}
