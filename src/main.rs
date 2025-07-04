use clap::Parser;
use sa::Args;
use sa::analysis::{calculate_metaspace, calculate_safety};
use sa::utils::{print_configuration, print_safety_report};
use std::collections::HashMap;

fn main() {
    let args = Args::parse();

    // 磁盘速度映射
    let disk_speeds: HashMap<&str, (f64, f64)> = [
        ("sata_hdd", (120.0, 100.0)),
        ("sata_ssd", (300.0, 250.0)),
        ("nvme", (1500.0, 1200.0)),
    ]
    .iter()
    .cloned()
    .collect();

    // 验证磁盘类型
    if !disk_speeds.contains_key::<str>(args.disk_type.as_str()) {
        eprintln!("错误: 不支持的磁盘类型. 可用选项: sata_hdd, sata_ssd, nvme");
        std::process::exit(1);
    }

    // 获取磁盘速度
    let (disk_read_speed, disk_write_speed) = disk_speeds[args.disk_type.as_str()];

    // 1. 计算内存分配
    let direct_mem_gb = (args.total_ram * 0.08).max(1.0);
    let heap_mem_gb = (args.total_ram * 0.35).max(4.0);

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
}
