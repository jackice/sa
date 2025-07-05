use anyhow::Context;
use clap::Parser;
use sa::Args;
use sa::analysis::{calculate_metaspace, calculate_safety};
use sa::config;
use sa::utils::{print_configuration, print_safety_report, print_system_limits};

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("启动文件传输系统分析工具");
    let args = Args::parse();

    // 获取磁盘配置
    let configs = config::get_disk_configs().read().unwrap();
    let disk_config = configs
        .get(args.disk_type.as_str())
        .context("无效的磁盘类型")?;
    let disk_read_speed = disk_config.read_speed;
    let disk_write_speed = disk_config.write_speed;

    // 1. 计算内存分配
    // 根据应用类型动态调整内存分配
    let (direct_ratio, heap_ratio) = match args.complexity.as_str() {
        "low" => (0.06, 0.4),    // 低复杂度应用需要更多堆
        "high" => (0.12, 0.3),   // 高IO应用需要更多直接内存
        _ => (0.08, 0.35)        // 默认比例
    };
    // 保证最小可用内存
    let direct_mem_gb = (args.total_ram * direct_ratio).max(1.0);
    let heap_mem_gb = (args.total_ram * heap_ratio).max(4.0);
    // 保留10%给JVM Native内存(线程栈等)
    let _native_mem_gb = args.total_ram * 0.1;
    log::debug!(
        "内存分配计算: 总内存={}GB, 直接内存={:.1}GB, 堆内存={:.1}GB",
        args.total_ram,
        direct_mem_gb,
        heap_mem_gb
    );

    // 2. 动态计算元空间大小
    let metaspace_size_mb = calculate_metaspace(&args);

    // 3. 计算安全系数
    let safety = calculate_safety(&args, direct_mem_gb, heap_mem_gb);

    // 1. 打印系统配置和基础分析
    print_configuration(
        &args,
        direct_mem_gb,
        heap_mem_gb,
        metaspace_size_mb,
        disk_read_speed,
        disk_write_speed,
    );

    // 2. 打印系统极限评估
    print_system_limits(&safety);

    // 3. 打印场景模拟分析
    sa::analysis::print_scenarios(&safety);

    // 4. 打印安全性报告
    print_safety_report(&safety);

    // 5. 计算并打印性能报告
    let performance = sa::analysis::performance::calculate_performance(
        &args,
        disk_config,
        direct_mem_gb,
        heap_mem_gb,
    );
    sa::utils::print_performance_report(&performance);

    // 6. 打印JVM配置建议
    sa::analysis::print_jvm_recommendations(
        &args,
        direct_mem_gb,
        heap_mem_gb,
        metaspace_size_mb,
        &safety,
        &performance,
    );

    // 9. 生成markdown报告
    if args.generate_markdown {
        let report_ctx = sa::utils::ReportContext {
            args: &args,
            direct_mem_gb,
            heap_mem_gb,
            metaspace_size_mb,
            disk_read_speed,
            disk_write_speed,
            safety: &safety,
            performance: &performance,
        };
        sa::utils::generate_markdown_report(&report_ctx)?;
        log::info!("Markdown报告已生成: sa_report.md");
    }

    Ok(())
}
