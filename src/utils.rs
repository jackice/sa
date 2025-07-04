use colored::Colorize;

pub fn print_configuration(
    args: &crate::args::Args,
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
        ("应用复杂度", args.complexity.to_string()),
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

pub fn print_safety_report(safety: &crate::analysis::SafetyAnalysis) {
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
    println!("\n  {}(0-1,越高越安全):", "内存安全系数".cyan());

    print_safety_bar("堆内存安全", safety.heap_safety);
    print_safety_bar("直接内存安全", safety.direct_mem_safety);

    // 防护建议
    if !safety.recommendations.is_empty() {
        println!("\n  {}:", "优化建议".cyan());
        for rec in &safety.recommendations {
            println!("    - {rec}");
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

// 扩展trait用于重复字符串
pub trait Repeated {
    fn repeated(&self, times: usize) -> String;
}

impl Repeated for &str {
    fn repeated(&self, times: usize) -> String {
        self.repeat(times)
    }
}

impl Repeated for colored::ColoredString {
    fn repeated(&self, times: usize) -> String {
        let colored_str = self.clone();
        let mut result = String::new();
        for _ in 0..times {
            result.push_str(&colored_str.to_string());
        }
        result
    }
}
