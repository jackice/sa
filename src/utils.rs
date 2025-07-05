use colored::Colorize;
use std::fs::File;
use std::io::Write;

/// 报告生成上下文
pub struct ReportContext<'a> {
    pub args: &'a crate::args::Args,
    pub direct_mem_gb: f64,
    pub heap_mem_gb: f64,
    pub metaspace_size_mb: i32,
    pub disk_read_speed: f64,
    pub disk_write_speed: f64,
    pub safety: &'a crate::analysis::SafetyAnalysis,
    pub performance: &'a crate::analysis::performance::PerformanceReport,
}

/// 生成markdown报告
pub fn generate_markdown_report(ctx: &ReportContext) -> anyhow::Result<()> {
    let mut file = File::create("sa_report.md")?;

    // 1. 标题和基本信息
    writeln!(file, "# 文件传输系统分析报告")?;
    writeln!(
        file,
        "> 生成时间: {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    )?;

    // 2. 系统配置
    writeln!(file, "## 系统配置")?;
    writeln!(file, "| 配置项 | 值 |")?;
    writeln!(file, "|--------|----|")?;
    writeln!(file, "| 服务器内存 | {:.1} GB |", ctx.args.total_ram)?;
    writeln!(file, "| CPU核心数 | {} |", ctx.args.cpu_cores)?;
    writeln!(file, "| 网络带宽 | {:.1} Gbps |", ctx.args.net_gbps)?;
    writeln!(
        file,
        "| 磁盘类型 | {} (读: {:.0} MB/s, 写: {:.0} MB/s) |",
        ctx.args.disk_type, ctx.disk_read_speed, ctx.disk_write_speed
    )?;
    writeln!(file, "| 平均文件大小 | {:.1} MB |", ctx.args.avg_file_size)?;
    writeln!(file, "| 预期并发连接 | {} |", ctx.args.expected_connections)?;
    writeln!(file, "| 突发流量倍数 | {}x |", ctx.args.burst_factor)?;
    writeln!(file, "| 应用复杂度 | {} |\n", ctx.args.complexity)?;

    // 3. 内存配置建议
    writeln!(file, "## 内存配置建议")?;
    writeln!(file, "- 推荐堆内存: {:.1} GB", ctx.heap_mem_gb)?;
    writeln!(file, "- 推荐直接内存: {:.1} GB", ctx.direct_mem_gb)?;
    writeln!(file, "- 元空间大小: {} MB\n", ctx.metaspace_size_mb)?;

    // 4. 安全性分析
    writeln!(file, "## 安全性分析")?;
    writeln!(file, "- 整体风险等级: **{}**", ctx.safety.risk_level)?;
    writeln!(
        file,
        "- 堆内存安全系数: {:.0}%",
        ctx.safety.heap_safety * 100.0
    )?;
    writeln!(
        file,
        "- 直接内存安全系数: {:.0}%",
        ctx.safety.direct_mem_safety * 100.0
    )?;

    // 安全系数图表
    writeln!(file, "\n### 内存安全系数图表")?;
    writeln!(file, "```")?;
    writeln!(file, "堆内存安全: {}", safety_bar(ctx.safety.heap_safety))?;
    writeln!(
        file,
        "直接内存安全: {}",
        safety_bar(ctx.safety.direct_mem_safety)
    )?;
    writeln!(file, "```\n")?;

    // 5. 场景模拟
    writeln!(file, "## 场景模拟")?;
    writeln!(
        file,
        "| 场景 | 连接数 | 文件大小(MB) | 堆内存(GB) | 直接内存(GB) | 状态 |"
    )?;
    writeln!(
        file,
        "|------|--------|--------------|------------|--------------|------|"
    )?;
    for scenario in &ctx.safety.scenarios {
        writeln!(
            file,
            "| {} | {} | {:.1} | {:.2} | {:.2} | {} |",
            scenario.name,
            scenario.connections,
            scenario.file_size,
            scenario.heap_usage,
            scenario.direct_mem_usage,
            String::from_utf8_lossy(&strip_ansi_escapes::strip(&scenario.status))
                .replace("✅", "✔️")
                .replace("⚠️", "⚠")
                .replace("🔥", "✖️")
        )?;
    }

    // 状态说明
    writeln!(file, "\n**状态说明:**")?;
    writeln!(file, "- ✔️ 安全: <70% 内存使用")?;
    writeln!(file, "- ⚠ 警告: 70-85% 内存使用")?;
    writeln!(file, "- ✖️ 危险: >85% 内存使用\n")?;

    // 6. JVM配置建议
    writeln!(file, "## JVM配置建议")?;
    writeln!(file, "```")?;
    writeln!(file, "# 基础配置")?;
    writeln!(
        file,
        "-Xms{}g -Xmx{}g",
        ctx.heap_mem_gb as i32, ctx.heap_mem_gb as i32
    )?;
    writeln!(
        file,
        "-XX:MaxDirectMemorySize={}g",
        ctx.direct_mem_gb as i32
    )?;
    writeln!(file, "-XX:MaxMetaspaceSize={}m", ctx.metaspace_size_mb)?;
    writeln!(file, "-XX:ReservedCodeCacheSize=256m")?;

    writeln!(file, "\n# 内存防护增强")?;
    writeln!(file, "-XX:+UseG1GC")?;
    writeln!(file, "-XX:MaxGCPauseMillis=200")?;
    writeln!(
        file,
        "-XX:ParallelGCThreads={}",
        (ctx.args.cpu_cores as f64 * 0.5).ceil() as i32
    )?;
    writeln!(
        file,
        "-XX:ConcGCThreads={}",
        (ctx.args.cpu_cores as f64 * 0.25).ceil() as i32
    )?;

    if ctx.safety.direct_mem_safety < 0.4 {
        writeln!(
            file,
            "-Djdk.nio.maxCachedBufferSize=131072  # 降低缓存阈值至128KB"
        )?;
    } else {
        writeln!(
            file,
            "-Djdk.nio.maxCachedBufferSize=262144  # 256KB缓存阈值"
        )?;
    }

    if ctx.args.enable_memory_guard {
        writeln!(file, "-Dapp.memory.guard.enabled=true")?;
        writeln!(
            file,
            "-Dapp.memory.guard.direct.threshold={:.1}g",
            ctx.direct_mem_gb * 0.85
        )?;
        writeln!(
            file,
            "-Dapp.memory.guard.heap.threshold={:.1}g",
            ctx.heap_mem_gb * 0.8
        )?;
    }

    // 元空间优化（针对高复杂度应用）
    if ctx.args.complexity == "high" {
        writeln!(file, "\n# 元空间优化（高复杂度应用）")?;
        writeln!(file, "-XX:+UseCompressedClassPointers")?;
        writeln!(
            file,
            "-XX:CompressedClassSpaceSize={}m",
            (ctx.metaspace_size_mb as f32 * 0.4).max(256.0) as i32
        )?;
        writeln!(file, "-XX:+UnlockExperimentalVMOptions")?;
        writeln!(file, "-XX:+UseZGC  # 可选：针对大堆内存使用ZGC")?;
    }

    // 监控配置
    writeln!(file, "\n# 监控与诊断")?;
    writeln!(file, "-XX:NativeMemoryTracking=detail")?;
    writeln!(file, "-XX:+PrintGCDetails -XX:+PrintGCDateStamps")?;
    writeln!(file, "-XX:+HeapDumpOnOutOfMemoryError")?;
    writeln!(file, "-XX:HeapDumpPath=/var/log/jvm_dumps")?;

    // 大文件优化
    if ctx.args.avg_file_size > 50.0 {
        writeln!(file, "\n# 大文件优化")?;
        writeln!(file, "-Djdk.nio.enableFastFileTransfer=true")?;
        writeln!(file, "-Dapp.file.maxChunkSize=2097152  # 2MB分块")?;
        writeln!(file, "-Dapp.file.useDirectIO=true")?;
    }
    writeln!(file, "```\n")?;

    // 7. 性能分析
    writeln!(file, "## 性能分析")?;
    for scenario in &ctx.performance.scenarios {
        writeln!(
            file,
            "### {} (平均文件大小: {}MB)",
            scenario.name, scenario.avg_file_size
        )?;

        writeln!(file, "\n#### 资源限制分析")?;
        writeln!(file, "| 资源类型 | 限制因素 | 最大并发量 | QPS |")?;
        writeln!(file, "|----------|----------|------------|-----|")?;
        for resource in &scenario.resources {
            let limit_mark = if resource.limiting_factor { "✓" } else { "" };
            writeln!(
                file,
                "| {} | {} | {} | {} |",
                resource.name,
                limit_mark,
                resource.max_connections,
                resource.qps.map_or("-".to_string(), |q| q.to_string())
            )?;
        }

        writeln!(
            file,
            "\n**最终能力:** {}并发 {} QPS",
            scenario.final_capacity.max_connections,
            scenario.final_capacity.qps.unwrap_or(0)
        )?;

        writeln!(file, "\n**关键发现:**")?;
        for finding in &scenario.key_findings {
            writeln!(file, "- {finding}")?;
        }
        writeln!(file)?;
    }

    // 8. 测试建议
    writeln!(file, "## 性能测试建议")?;
    writeln!(file, "- 线程数: {}", ctx.performance.test_config.threads)?;
    writeln!(file, "- 测试时长: {}", ctx.performance.test_config.duration)?;
    writeln!(file, "- 加压时间: {}", ctx.performance.test_config.ramp_up)?;
    writeln!(
        file,
        "- 目标吞吐量: {:.1} QPS",
        ctx.performance.test_config.throughput_goal
    )?;

    // 测试脚本示例
    writeln!(file, "\n### 测试脚本示例")?;
    for (i, script) in ctx
        .performance
        .test_config
        .script_examples
        .iter()
        .enumerate()
    {
        writeln!(file, "#### 示例 {}:", i + 1)?;
        writeln!(file, "```bash")?;
        writeln!(file, "{script}")?;
        writeln!(file, "```")?;
    }

    // 9. 优化建议
    if !ctx.safety.recommendations.is_empty() {
        writeln!(file, "\n## 优化建议")?;
        for rec in &ctx.safety.recommendations {
            writeln!(file, "- {rec}")?;
        }
    }

    Ok(())
}

fn safety_bar(value: f64) -> String {
    let width = 30;
    let fill = (value * width as f64) as usize;
    let empty = width - fill;
    format!(
        "[{}{}] {:.0}%",
        "■".repeat(fill),
        " ".repeat(empty),
        value * 100.0
    )
}

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

pub fn print_performance_report(report: &crate::analysis::performance::PerformanceReport) {
    println!(
        "\n{}{}",
        "▬".magenta().bold().reversed(),
        " 全链路性能分析报告 ".magenta().bold().reversed()
    );
    println!("{}", "▬".magenta().bold().repeated(50));

    for scenario in &report.scenarios {
        println!(
            "\n  {} (平均文件大小: {}MB)",
            scenario.name.bold(),
            scenario.avg_file_size
        );

        println!(
            "  {:<12} {:<12} {:<12} {:<12}",
            "资源类型".cyan(),
            "限制因素".cyan(),
            "最大并发量".cyan(),
            "QPS".cyan()
        );

        for resource in &scenario.resources {
            let limit_mark = if resource.limiting_factor { "✓" } else { "" };
            println!(
                "  {:<12} {:<12} {:<12} {:<12}",
                resource.name,
                limit_mark,
                resource.max_connections,
                resource.qps.map_or("-".to_string(), |q| q.to_string())
            );
        }

        println!(
            "\n  {}: {}并发 {} QPS",
            "最终能力".cyan().bold(),
            scenario.final_capacity.max_connections,
            scenario.final_capacity.qps.unwrap_or(0)
        );

        println!("\n  {}:", "关键发现".cyan());
        for finding in &scenario.key_findings {
            println!("    - {finding}");
        }
    }

    println!("\n  {}:", "性能测试建议".cyan().bold());
    println!("    - {}: {}", "线程数".cyan(), report.test_config.threads);
    println!(
        "    - {}: {}",
        "测试时长".cyan(),
        report.test_config.duration
    );
    println!(
        "    - {}: {}",
        "加压时间".cyan(),
        report.test_config.ramp_up
    );
    println!(
        "    - {}: {:.1} QPS",
        "目标吞吐量".cyan(),
        report.test_config.throughput_goal
    );

    println!("\n  {}:", "测试脚本示例".cyan().bold());
    for (i, script) in report.test_config.script_examples.iter().enumerate() {
        println!("    {}. {}", i + 1, script);
    }
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
