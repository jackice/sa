pub mod jvm;
pub mod safety;
pub mod scenarios;

pub use jvm::print_jvm_recommendations;
pub use safety::Scenario;
pub use safety::{SafetyAnalysis, calculate_safety};
pub use scenarios::print_scenarios;

pub fn calculate_metaspace(args: &crate::args::Args) -> i32 {
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
    total.clamp(256.0, 2048.0).ceil() as i32
}
