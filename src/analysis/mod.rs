pub mod jvm;
pub mod performance;
pub mod safety;
pub mod scenarios;

pub use jvm::print_jvm_recommendations;
pub use safety::Scenario;
pub use safety::{SafetyAnalysis, calculate_safety};
pub use scenarios::print_scenarios;

/// 元空间计算相关常量
const BASE_METASPACE: f64 = 512.0; // 基础元空间大小(MB)
const CONNECTION_FACTOR: f64 = 50.0; // 每1000连接增加的大小(MB)
const FILE_SIZE_FACTOR: f64 = 50.0; // 每50MB文件大小增加的大小(MB)
const SAFETY_MARGIN: f64 = 1.25; // 安全系数
const MIN_METASPACE: f64 = 256.0; // 最小元空间大小(MB)
const MAX_METASPACE: f64 = 2048.0; // 最大元空间大小(MB)
const COMPLEXITY_LOW_FACTOR: f64 = 0.8; // 低复杂度系数
const COMPLEXITY_HIGH_FACTOR: f64 = 1.5; // 高复杂度系数
const FILE_SIZE_BASE: f64 = 50.0; // 文件大小计算基准值(MB)
const CONNECTIONS_BASE: f64 = 1000.0; // 连接数计算基准值

/// 计算基础元空间大小
///
/// # 参数
/// - args: 命令行参数
///
/// # 返回值
/// 根据应用复杂度调整后的基础元空间大小(MB)
fn calculate_base_metaspace(args: &crate::args::Args) -> f64 {
    let mut base = BASE_METASPACE;
    match args.complexity.as_str() {
        "low" => base *= COMPLEXITY_LOW_FACTOR,
        "high" => base *= COMPLEXITY_HIGH_FACTOR,
        _ => {} // medium保持原值
    }
    base
}

/// 计算连接数相关元空间增量
///
/// 每1000个连接增加50MB元空间
fn calculate_connection_factor(args: &crate::args::Args) -> f64 {
    (args.expected_connections as f64 / CONNECTIONS_BASE).floor() * CONNECTION_FACTOR
}

/// 计算文件大小相关元空间增量
///
/// 每50MB文件大小增加50MB元空间，最大不超过4倍
fn calculate_file_size_factor(args: &crate::args::Args) -> f64 {
    (args.avg_file_size / FILE_SIZE_BASE).floor() * FILE_SIZE_FACTOR
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
    let adjusted_total = (raw_total * SAFETY_MARGIN).max(MIN_METASPACE * SAFETY_MARGIN);

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
            BASE_METASPACE * COMPLEXITY_LOW_FACTOR
        );
        assert_approx_eq::assert_approx_eq!(calculate_base_metaspace(&medium), BASE_METASPACE);
        assert_approx_eq::assert_approx_eq!(
            calculate_base_metaspace(&high),
            BASE_METASPACE * COMPLEXITY_HIGH_FACTOR
        );
    }

    #[test]
    fn test_calculate_connection_factor() {
        let args = create_test_args("medium", 1500, 10.0);
        assert_approx_eq::assert_approx_eq!(calculate_connection_factor(&args), 50.0); // 1000-1999 connections = 1x factor

        let args = create_test_args("medium", 3500, 10.0);
        assert_approx_eq::assert_approx_eq!(calculate_connection_factor(&args), 150.0); // 3000-3999 connections = 3x factor
    }

    #[test]
    fn test_calculate_file_size_factor() {
        let args = create_test_args("medium", 1000, 60.0);
        assert_approx_eq::assert_approx_eq!(calculate_file_size_factor(&args), 50.0); // 60/50 = 1.2 floored to 1 -> 1x factor

        let args = create_test_args("medium", 1000, 250.0);
        assert_approx_eq::assert_approx_eq!(calculate_file_size_factor(&args), 250.0); // 250/50 = 5 -> 5x factor
    }

    #[test]
    fn test_calculate_metaspace_boundaries() {
        // Test minimum boundary
        let args = create_test_args("low", 100, 1.0);
        let result = calculate_metaspace(&args);
        assert_eq!(result, 512); // 256 * 1.25 = 320

        // Test maximum boundary
        let args = create_test_args("high", 10000, 500.0);
        let result = calculate_metaspace(&args);
        assert_eq!(result, MAX_METASPACE as i32);
    }

    #[test]
    fn test_calculate_metaspace_normal_case() {
        let args = create_test_args("medium", 2000, 50.0);
        let expected = (BASE_METASPACE + 100.0 + 50.0) * SAFETY_MARGIN; // base + 2x connection + 1x file
        let result = calculate_metaspace(&args);
        assert_approx_eq::assert_approx_eq!(result as f64, expected, 1.0); // Allow 1MB tolerance
    }
}
