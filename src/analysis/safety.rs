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
    pub heap_safety: f64,             // 堆内存安全系数 (0-1)
    pub direct_mem_safety: f64,       // 直接内存安全系数 (0-1)
    pub risk_level: String,           // 整体风险等级
    pub scenarios: Vec<Scenario>,     // 模拟场景
    pub recommendations: Vec<String>, // 优化建议
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
