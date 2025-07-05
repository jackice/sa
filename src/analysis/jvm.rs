use crate::utils::Repeated;
use crate::{SafetyAnalysis, args::Args, analysis::performance::PerformanceReport};
use colored::Colorize; // Bring trait implementation into scope

/// 基于全面分析生成最终JVM配置建议
pub fn print_jvm_recommendations(
    args: &Args,
    direct_mem_gb: f64,
    heap_mem_gb: f64,
    metaspace_size_mb: i32,
    safety: &SafetyAnalysis,
    _performance: &PerformanceReport,
) {
    // 1. 评估当前配置是否满足6个月稳定运行
    let meets_requirements = safety.theoretical_limits.estimated_uptime.contains("6-12个月") 
        || safety.theoretical_limits.estimated_uptime.contains("12个月+");

    // 2. 计算理论最大连接数(基于最严格限制资源)
    let max_sustainable_conn = safety.theoretical_limits.max_connections;
    let target_conn = args.expected_connections;
    let needs_scaling = target_conn > max_sustainable_conn;

    // 3. 打印配置摘要
    println!(
        "\n{}{}",
        "▬".green().bold().reversed(),
        " JVM配置建议 ".green().bold().reversed()
    );
    println!("{}", "▬".green().bold().repeated(50));

    println!("\n{}", "  # 系统能力评估".bold());
    println!("  - 当前配置理论最大连接数: {}", max_sustainable_conn);
    println!("  - 目标连接数: {}", target_conn);
    println!("  - 稳定运行预期: {}", safety.theoretical_limits.estimated_uptime);
    println!("  - 主要瓶颈资源: {}", safety.theoretical_limits.limiting_factor);

    if !meets_requirements {
        println!("\n{}", "  ❗ 警告: 当前配置无法满足6个月稳定运行要求".red().bold());
    }

    if needs_scaling {
        println!("\n{}", "  ⚠️ 注意: 目标连接数超过理论最大值".yellow().bold());
        println!("  - 需要调整资源配置或优化应用");
        println!("  - 理论可达到连接数: {}", max_sustainable_conn);
    }

    // 4. 生成最终配置建议
    println!("\n{}", "  # 最终JVM配置建议".bold());
    println!(
        "\n{}{}",
        "▬".green().bold().reversed(),
        " JVM配置建议 ".green().bold().reversed()
    );
    println!("{}", "▬".green().bold().repeated(50));

    // JDK版本兼容性评估
    println!("\n{}", "  # JDK版本兼容性".bold());
    if args.complexity == "high" {
        println!("  - 建议使用JDK 17+ (包含ZGC和元空间优化)");
    } else {
        println!("  - 最低要求: JDK 11");
        println!("  - 推荐版本: JDK 17+ (更好的性能与内存管理)");
    }
    
    println!("\n{}", "  ## 参数兼容性详情".bold());
    println!("  - 基础配置:");
    println!("    - -Xms/-Xmx: 所有版本支持");
    println!("    - -XX:MaxDirectMemorySize: JDK 6+ 支持");
    println!("    - -XX:MaxMetaspaceSize: JDK 8+ 支持 (JDK 7及以下使用-XX:MaxPermSize)");
    println!("    - -XX:ReservedCodeCacheSize: JDK 6+ 支持");
    
    println!("  - 内存防护增强:");
    println!("    - -XX:+UseG1GC: JDK 7u4+ 完全支持");
    println!("    - -XX:MaxGCPauseMillis: JDK 6u14+ 支持");
    println!("    - -XX:ParallelGCThreads/-XX:ConcGCThreads: JDK 6+ 支持");
    println!("    - -Djdk.nio.maxCachedBufferSize: JDK 7+ 支持");
    
    println!("  - 元空间优化:");
    println!("    - -XX:+UseCompressedClassPointers: JDK 6+ 支持64位系统");
    println!("    - -XX:CompressedClassSpaceSize: JDK 8+ 支持");
    println!("    - -XX:+UnlockExperimentalVMOptions: JDK 7+ 支持");
    println!("    - -XX:+UseZGC: JDK 11+ 支持 (JDK 15+ 生产可用)");
    
    println!("  - 监控配置:");
    println!("    - -XX:NativeMemoryTracking: JDK 8+ 支持");
    println!("    - -XX:+PrintGCDetails: JDK 6+ 支持 (JDK 9+ 使用-Xlog:gc*)");
    println!("    - -XX:+HeapDumpOnOutOfMemoryError: JDK 6+ 支持");
    
    println!("  - 大文件优化:");
    println!("    - -Djdk.nio.enableFastFileTransfer: JDK 9+ 支持");
    println!("    - DirectIO相关参数: 需要特定JDK实现或第三方库");

    // 基础配置(根据需求调整)
    let (final_heap, final_direct, server_ram_needed) = if needs_scaling {
        // 按比例扩大内存配置以达到目标
        let scale_factor = target_conn as f64 / max_sustainable_conn as f64;
        let new_heap = (heap_mem_gb * scale_factor).max(heap_mem_gb * 1.2);
        let new_direct = (direct_mem_gb * scale_factor).max(direct_mem_gb * 1.3);
        let total_ram_needed = (new_heap + new_direct) / 0.85; // 保留15%给系统
        
        (
            new_heap as i32,
            new_direct as i32,
            Some(total_ram_needed.ceil() as i32)
        )
    } else {
        (heap_mem_gb as i32, direct_mem_gb as i32, None)
    };

    println!("{}", "  ## 基础配置".bold());
    println!("  -Xms{}g -Xmx{}g  # {}", final_heap, final_heap, 
        if needs_scaling { "已按目标调整" } else { "基于当前负载" });
    println!("  -XX:MaxDirectMemorySize={}g  # {}", final_direct,
        if needs_scaling { "已按目标调整" } else { "基于当前负载" });
    println!("  -XX:MaxMetaspaceSize={metaspace_size_mb}m  # 动态计算值");
    println!("  -XX:ReservedCodeCacheSize=256m  # 固定值");

    // 添加容量说明
    println!("\n{}", "  ## 容量说明".bold());
    println!("  - 配置支持最大连接数: {}", max_sustainable_conn);
    if needs_scaling {
        println!("  - {}: 需要额外 {}% 资源以达到目标连接数", 
            "资源缺口".red(), 
            ((target_conn as f64 / max_sustainable_conn as f64 - 1.0) * 100.0) as i32);
        
        if let Some(ram_needed) = server_ram_needed {
            println!("  - {}: 建议服务器内存至少 {}GB (当前 {}GB)",
                "内存扩容建议".yellow(),
                ram_needed,
                args.total_ram as i32);
            
            // CPU核心建议 (每1000连接需要1核)
            let suggested_cores = (target_conn as f64 / 1000.0).ceil() as i32;
            if suggested_cores > args.cpu_cores as i32 {
                println!("  - {}: 建议CPU核心数 {} (当前 {})",
                    "CPU扩容建议".yellow(),
                    suggested_cores,
                    args.cpu_cores);
            }
            
            // 网络带宽建议 (每连接0.2Mbps)
            let suggested_bandwidth = (target_conn as f64 * 0.2 / 1000.0).ceil() as i32;
            if suggested_bandwidth > args.net_gbps as i32 {
                println!("  - {}: 建议网络带宽 {}Gbps (当前 {}Gbps)",
                    "网络扩容建议".yellow(),
                    suggested_bandwidth,
                    args.net_gbps);
            }
        }
    }

    // 内存防护增强
    println!("\n{}", "  # 内存防护增强".bold());
    println!("  -XX:+UseG1GC");
    println!("  -XX:MaxGCPauseMillis=200");
    println!(
        "  -XX:ParallelGCThreads={}",
        (args.cpu_cores as f64 * 0.5).ceil() as i32
    );
    println!(
        "  -XX:ConcGCThreads={}",
        (args.cpu_cores as f64 * 0.25).ceil() as i32
    );

    if safety.direct_mem_safety < 0.4 {
        println!("  -Djdk.nio.maxCachedBufferSize=131072  # 降低缓存阈值至128KB");
    } else {
        println!("  -Djdk.nio.maxCachedBufferSize=262144  # 256KB缓存阈值");
    }

    if args.enable_memory_guard {
        println!("  -Dapp.memory.guard.enabled=true");
        println!(
            "  -Dapp.memory.guard.direct.threshold={:.1}g",
            direct_mem_gb * 0.85
        );
        println!(
            "  -Dapp.memory.guard.heap.threshold={:.1}g",
            heap_mem_gb * 0.8
        );
    }

    // 元空间优化（针对高复杂度应用）
    if args.complexity == "high" {
        println!("\n{}", "  # 元空间优化（高复杂度应用）".bold());
        println!("  -XX:+UseCompressedClassPointers");
        println!(
            "  -XX:CompressedClassSpaceSize={}m",
            (metaspace_size_mb as f32 * 0.4).max(256.0) as i32
        );
        println!("  -XX:+UnlockExperimentalVMOptions");
        println!("  -XX:+UseZGC  # 可选：针对大堆内存使用ZGC");
    }

    // 监控配置
    println!("\n{}", "  # 监控与诊断".bold());
    println!("  -XX:NativeMemoryTracking=detail");
    println!("  -XX:+PrintGCDetails -XX:+PrintGCDateStamps");
    println!("  -XX:+HeapDumpOnOutOfMemoryError");
    println!("  -XX:HeapDumpPath=/var/log/jvm_dumps");
    println!("  -XX:+PrintClassHistogramBeforeFullGC");
    println!("  -XX:+PrintClassHistogramAfterFullGC");
    println!("  -XX:+PrintReferenceGC");
    println!("  -XX:+PrintTenuringDistribution");
    println!("  -XX:+UnlockDiagnosticVMOptions");
    println!("  -XX:+LogCompilation");
    println!("  -XX:LogFile=/var/log/jvm_compilation.log");

    // 大文件优化
    if args.avg_file_size > 50.0 {
        println!("\n{}", "  # 大文件优化".bold());
        println!("  -Djdk.nio.enableFastFileTransfer=true");
        println!("  -Dapp.file.maxChunkSize=2097152  # 2MB分块");
        println!("  -Dapp.file.useDirectIO=true");
    }

    println!("\n{}", "  # 启动命令示例".bold());
    println!("  java \\");
    println!("    -Xms{0}g -Xmx{0}g \\", heap_mem_gb as i32);
    println!("    -XX:MaxDirectMemorySize={}g \\", direct_mem_gb as i32);
    println!("    -XX:MaxMetaspaceSize={metaspace_size_mb}m \\");
    println!("    -XX:ReservedCodeCacheSize=256m \\");
    println!("    -jar your-application.jar");
}
