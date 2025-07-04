use crate::utils::Repeated;
use crate::{SafetyAnalysis, args::Args};
use colored::Colorize; // Bring trait implementation into scope

pub fn print_jvm_recommendations(
    args: &Args,
    direct_mem_gb: f64,
    heap_mem_gb: f64,
    metaspace_size_mb: i32,
    safety: &SafetyAnalysis,
) {
    println!(
        "\n{}{}",
        "▬".green().bold().reversed(),
        " JVM配置建议 ".green().bold().reversed()
    );
    println!("{}", "▬".green().bold().repeated(50));

    // 基础配置
    println!("{}", "  # 基础配置".bold());
    println!("  -Xms{}g -Xmx{}g", heap_mem_gb as i32, heap_mem_gb as i32);
    println!("  -XX:MaxDirectMemorySize={}g", direct_mem_gb as i32);
    println!("  -XX:MaxMetaspaceSize={metaspace_size_mb}m  # 动态计算值");
    println!("  -XX:ReservedCodeCacheSize=256m");

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
