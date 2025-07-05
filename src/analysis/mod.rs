pub mod jvm;
pub mod performance;
pub mod safety;
pub mod scenarios;

pub use jvm::print_jvm_recommendations;
pub use safety::Scenario;
pub use safety::{SafetyAnalysis, calculate_safety};
pub use scenarios::print_scenarios;

use crate::args::Args;

/// 元空间计算模型 (基于文件类型和连接数)
const BASE_METASPACE: f64 = 256.0; // 基础元空间大小(MB)
const CONNECTION_FACTOR: f64 = 30.0; // 每1000连接增加的大小(MB)
const FILE_SIZE_FACTOR: f64 = 20.0; // 每100MB文件大小增加的大小(MB) 
const THREAD_FACTOR: f64 = 1.0; // 每个线程增加的大小(MB)
const MIN_METASPACE: f64 = 128.0; // 最小元空间大小(MB)
const MAX_METASPACE: f64 = 3072.0; // 最大元空间大小(MB)
const CONNECTIONS_BASE: f64 = 1000.0; // 连接数计算基准值

/// 根据文件类型获取复杂度因子
fn get_complexity_factor(args: &Args) -> f64 {
    match (args.complexity.as_str(), args.avg_file_size) {
        ("high", _) => 1.5,  // 高复杂度应用
        ("low", _) => 0.8,   // 低复杂度
        (_, fs) if fs > 50.0 => 1.3,  // 大文件
        _ => 1.0  // 中等复杂度
    }
}

/// 根据文件类型获取安全边际
fn get_safety_margin(args: &Args) -> f64 {
    match args.avg_file_size {
        fs if fs > 100.0 => 1.5,  // 超大文件
        fs if fs > 50.0 => 1.4,   // 大文件
        _ => 1.3  // 普通文件
    }
}

/// 计算基础元空间大小(考虑文件类型和线程数)
fn calculate_base_metaspace(args: &crate::args::Args) -> f64 {
    let base = BASE_METASPACE * get_complexity_factor(args);
    // 每个线程需要约1MB元空间
    let threads = (args.cpu_cores * 2) as f64; // IO密集型应用通常需要2*CPU核心数的线程
    base + threads * THREAD_FACTOR
}

/// 计算连接数相关元空间增量
///
/// 每1000个连接增加50MB元空间
fn calculate_connection_factor(args: &crate::args::Args) -> f64 {
    (args.expected_connections as f64 / CONNECTIONS_BASE).floor() * CONNECTION_FACTOR
}

/// 计算文件大小相关元空间增量(非线性增长)
fn calculate_file_size_factor(args: &crate::args::Args) -> f64 {
    // 小文件(<=10MB)固定增加20MB
    if args.avg_file_size <= 10.0 {
        return 20.0;
    }
    // 中等文件(10-100MB)对数增长
    if args.avg_file_size <= 100.0 {
        return (args.avg_file_size.ln() * 10.0).round();
    }
    // 大文件(>100MB)线性增长但增速减缓
    (args.avg_file_size / 100.0).floor() * FILE_SIZE_FACTOR
}

/// 计算推荐的元空间大小
///
/// 综合考虑基础值、连接数、文件大小和安全系数，
/// 返回一个在合理范围内的元空间大小建议值
///
/// # 参数
/// - args: 命令行参数
///
/// # 返回值
/// 推荐的元空间大小(MB)
pub fn calculate_metaspace(args: &crate::args::Args) -> i32 {
    let base = calculate_base_metaspace(args);
    let connection_factor = calculate_connection_factor(args);
    let file_size_factor = calculate_file_size_factor(args);

    let raw_total = base + connection_factor + file_size_factor;

    // Apply minimum boundary after safety margin
    let safety_margin = get_safety_margin(args);
    let adjusted_total = (raw_total * safety_margin).max(MIN_METASPACE * safety_margin);

    adjusted_total.min(MAX_METASPACE).ceil() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Args;

    fn create_test_args(complexity: &str, connections: usize, file_size: f64) -> Args {
        Args {
            complexity: complexity.to_string(),
            expected_connections: connections,
            avg_file_size: file_size,
            ..Default::default()
        }
    }

    #[test]
    fn test_calculate_base_metaspace() {
        let low = create_test_args("low", 1000, 10.0);
        let medium = create_test_args("medium", 1000, 10.0);
        let high = create_test_args("high", 1000, 10.0);

        assert_approx_eq::assert_approx_eq!(
            calculate_base_metaspace(&low),
            BASE_METASPACE * 0.8  // matches get_complexity_factor("low")
        );
        assert_approx_eq::assert_approx_eq!(calculate_base_metaspace(&medium), BASE_METASPACE);
        assert_approx_eq::assert_approx_eq!(
            calculate_base_metaspace(&high),
            BASE_METASPACE * 1.5  // matches get_complexity_factor("high")
        );
    }

    #[test]
    fn test_calculate_connection_factor() {
        let args = create_test_args("medium", 1500, 10.0);
        assert_approx_eq::assert_approx_eq!(calculate_connection_factor(&args), 30.0); // 1000-1999 connections = 1x factor (30MB)

        let args = create_test_args("medium", 3500, 10.0);
        assert_approx_eq::assert_approx_eq!(calculate_connection_factor(&args), 90.0); // 3000-3999 connections = 3x factor (90MB)
    }

    #[test]
    fn test_calculate_file_size_factor() {
        let args = create_test_args("medium", 1000, 60.0);
        assert_approx_eq::assert_approx_eq!(calculate_file_size_factor(&args), 41.0); // ln(60)*10 ≈ 41

        let args = create_test_args("medium", 1000, 250.0);
        assert_approx_eq::assert_approx_eq!(calculate_file_size_factor(&args), 40.0); // 250/100 = 2.5 floored to 2 -> 2*20=40
    }

    #[test]
    fn test_calculate_metaspace_boundaries() {
        // Test minimum boundary
        let args = create_test_args("low", 100, 1.0);
        let result = calculate_metaspace(&args);
        assert!(result >= (MIN_METASPACE * 1.3) as i32); // With safety margin
        assert!(result <= MAX_METASPACE as i32);

        // Test maximum boundary 
        let args = create_test_args("high", 10000, 500.0);
        let result = calculate_metaspace(&args);
        assert!(result <= MAX_METASPACE as i32);

        // Test thread factor
        let args = Args {
            cpu_cores: 16,
            ..create_test_args("medium", 1000, 10.0)
        };
        let result = calculate_metaspace(&args);
        assert!(result > 300); // Should include thread overhead
    }

    #[test]
    fn test_calculate_metaspace_normal_case() {
        let args = create_test_args("medium", 2000, 50.0);
        let safety_margin = get_safety_margin(&args);
        let base = BASE_METASPACE * get_complexity_factor(&args);
        let connection_factor = calculate_connection_factor(&args);
        let file_size_factor = calculate_file_size_factor(&args);
        let expected = (base + connection_factor + file_size_factor) * safety_margin;
        let result = calculate_metaspace(&args);
        assert_approx_eq::assert_approx_eq!(result as f64, expected, 1.0); // Allow 1MB tolerance
    }
}
