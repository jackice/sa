use clap::Parser;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("不支持的磁盘类型: {0}")]
    InvalidDiskType(String),
    #[error("无效的内存值: {0}")]
    InvalidMemoryValue(f64),
    #[error("无效的连接数: {0}")]
    InvalidConnectionCount(usize),
}

/// 文件上传下载系统性能与安全性分析工具
#[derive(Parser, Debug, Default)]
#[clap(version = "3.2", author = "System Safety Analyst")]
pub struct Args {
    /// 服务器总内存(GB) [必须大于0]
    #[clap(short, long, default_value = "32", value_parser = validate_positive_float)]
    pub total_ram: f64,

    /// CPU核心数
    #[clap(short = 'c', long, default_value = "16")]
    pub cpu_cores: usize,

    /// 网络带宽(Gbps)
    #[clap(short = 'w', long, default_value = "1")]
    pub net_gbps: f64,

    /// 磁盘类型 [sata_hdd, sata_ssd, nvme]
    #[clap(short = 'd', long, default_value = "sata_ssd", value_parser = validate_disk_type)]
    pub disk_type: String,

    /// 平均文件大小(MB)
    #[clap(short = 'f', long, default_value = "10")]
    pub avg_file_size: f64,

    /// 预期最大并发连接数
    #[clap(short = 'n', long, default_value = "1000")]
    pub expected_connections: usize,

    /// 最大突发流量倍数
    #[clap(short = 'b', long, default_value = "3")]
    pub burst_factor: f64,

    /// 是否启用内存防护 [true, false]
    #[clap(short = 'p', long, default_value = "true")]
    pub enable_memory_guard: bool,

    /// 应用复杂度级别 [low, medium, high]
    #[clap(short = 'l', long, default_value = "medium")]
    pub complexity: String,
}
fn validate_positive_float(s: &str) -> Result<f64, String> {
    let val: f64 = s.parse().map_err(|_| format!("`{s}` 不是有效的浮点数"))?;
    if val > 0.0 {
        Ok(val)
    } else {
        Err(format!("值必须大于0, 但得到 {val}"))
    }
}

fn validate_disk_type(s: &str) -> Result<String, String> {
    match s {
        "sata_hdd" | "sata_ssd" | "nvme" => Ok(s.to_string()),
        _ => Err(format!("不支持的磁盘类型: {s}. 可用选项: sata_hdd, sata_ssd, nvme")),
    }
}
