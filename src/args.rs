use clap::Parser;

/// 文件上传下载系统性能与安全性分析工具
#[derive(Parser, Debug)]
#[clap(version = "3.2", author = "System Safety Analyst")]
pub struct Args {
    /// 服务器总内存(GB)
    #[clap(short, long, default_value = "32")]
    pub total_ram: f64,

    /// CPU核心数
    #[clap(short = 'c', long, default_value = "16")]
    pub cpu_cores: usize,

    /// 网络带宽(Gbps)
    #[clap(short = 'w', long, default_value = "1")]
    pub net_gbps: f64,

    /// 磁盘类型 [sata_hdd, sata_ssd, nvme]
    #[clap(short = 'd', long, default_value = "sata_ssd")]
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
