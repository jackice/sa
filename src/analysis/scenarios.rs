use crate::analysis::SafetyAnalysis;
use crate::utils::Repeated;
use colored::Colorize;

pub fn print_scenarios(safety: &SafetyAnalysis) {
    println!(
        "\n{}{}",
        "▬".magenta().bold().reversed(),
        " 负载场景模拟 ".magenta().bold().reversed()
    );
    println!("{}", "▬".blue().bold().repeated(50));

    println!(
        "  {:<18} {:<12} {:<12} {:<12} {:<12} {:<10}",
        "场景".cyan(),
        "连接数".cyan(),
        "文件大小".cyan(),
        "堆内存".cyan(),
        "直接内存".cyan(),
        "状态".cyan()
    );

    for scenario in &safety.scenarios {
        println!(
            "  {:<18} {:<12} {:<12.1} {:<12.2} {:<12.2} {}",
            scenario.name,
            scenario.connections,
            scenario.file_size,
            scenario.heap_usage,
            scenario.direct_mem_usage,
            scenario.status
        );
    }

    // 解释状态标识
    println!("\n  {}: <70% 内存使用", "✅ 安全".green());
    println!("  {}: 70-85% 内存使用", "⚠️ 警告".yellow());
    println!("  {}: >85% 内存使用", "🔥 危险".red());
}
