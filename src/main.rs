use anyhow::Context;
use clap::Parser;
use sa::Args;
use sa::analysis::{calculate_metaspace, calculate_safety};
use sa::config;
use sa::utils::{print_configuration, print_safety_report};

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
    let direct_mem_gb = (args.total_ram * 0.08).max(1.0);
    let heap_mem_gb = (args.total_ram * 0.35).max(4.0);
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

    // 4. 打印系统配置
    print_configuration(
        &args,
        direct_mem_gb,
        heap_mem_gb,
        metaspace_size_mb,
        disk_read_speed,
        disk_write_speed,
    );

    // 5. 打印安全性报告
    print_safety_report(&safety);

    // 6. 打印场景模拟
    sa::analysis::print_scenarios(&safety);

    // 7. 打印JVM配置建议
    sa::analysis::print_jvm_recommendations(
        &args,
        direct_mem_gb,
        heap_mem_gb,
        metaspace_size_mb,
        &safety,
    );

    Ok(())
}
