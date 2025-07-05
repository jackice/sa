use crate::args::Args;
use crate::config::DiskConfig;

/// 资源瓶颈分析
#[derive(Clone)]
pub struct ResourceLimit {
    pub name: String,          // 资源名称
    pub limiting_factor: bool,  // 是否为当前限制因素
    pub max_connections: usize, // 最大并发量
    pub qps: Option<usize>,     // 每秒查询数(对大文件可能为None)
}

/// 性能分析结果
pub struct PerformanceReport {
    pub scenarios: Vec<ScenarioAnalysis>, // 不同场景分析
    pub test_config: TestConfig,          // 性能测试建议配置
}

/// 场景分析
pub struct ScenarioAnalysis {
    pub name: String,           // 场景名称
    pub avg_file_size: f64,     // 平均文件大小(MB)
    pub resources: Vec<ResourceLimit>, // 各资源限制
    pub final_capacity: ResourceLimit, // 最终能力
    pub key_findings: Vec<String>, // 关键发现
}

/// 性能测试建议配置
pub struct TestConfig {
    pub threads: usize,           // 建议线程数
    pub duration: String,         // 测试时长建议
    pub ramp_up: String,          // 加压时间建议
    pub throughput_goal: f64,     // 目标吞吐量(MB/s)
    pub script_examples: Vec<String>, // 测试脚本示例
}

/// 计算性能报告
pub fn calculate_performance(
    args: &Args,
    disk_config: &DiskConfig,
    direct_mem_gb: f64,
    heap_mem_gb: f64,
) -> PerformanceReport {
    // 计算内存限制的并发量
    let mem_per_conn = 0.5; // MB/连接(堆+直接内存)
    let mem_connections = ((direct_mem_gb + heap_mem_gb) * 1024.0 / mem_per_conn) as usize;

    // 定义要分析的场景
    let scenarios = vec![
        analyze_scenario("混合文件大小", 30.0, args, disk_config, mem_connections),
        analyze_scenario("小文件为主", 5.0, args, disk_config, mem_connections),
    ];

    // 生成性能测试建议
    let script_examples = vec![
        format!(
            "# 使用wrk进行混合文件测试\n\
            wrk -t{} -c{} -d{} -s upload_script.lua http://your-server/upload\n\n\
            # upload_script.lua\n\
            function init()\n\
                math.randomseed(os.time())\n\
                sizes = {{1, 5, 10, 30, 100}} -- MB\n\
            end\n\n\
            function request()\n\
                -- 随机选择文件大小\n\
                size = sizes[math.random(#sizes)]\n\
                file_path = \"test_files/\" .. size .. \"mb.dat\"\n\
                \n\
                -- 读取文件内容\n\
                local file = io.open(file_path, \"rb\")\n\
                local content = file:read(\"*all\")\n\
                file:close()\n\
                \n\
                -- 构造请求\n\
                wrk.headers[\"Content-Type\"] = \"application/octet-stream\"\n\
                wrk.headers[\"Content-Length\"] = #content\n\
                return wrk.format(\"POST\", \"/upload\", wrk.headers, content)\n\
            end",
            args.cpu_cores,
            args.expected_connections,
            "10m"
        ),
        format!(
            "# 使用ab进行固定大小文件测试\n\
            ab -n {} -c {} -T \"application/octet-stream\" -p test_files/10mb.dat http://your-server/upload",
            args.expected_connections * 100,
            args.expected_connections
        ),
    ];

    let test_config = TestConfig {
        threads: args.cpu_cores * 2,
        duration: "10m".to_string(),
        ramp_up: "1m".to_string(),
        throughput_goal: scenarios.iter()
            .map(|s| s.final_capacity.qps.unwrap_or(0) as f64)
            .fold(f64::INFINITY, |a, b| a.min(b)),
        script_examples,
    };

    PerformanceReport {
        scenarios,
        test_config,
    }
}

fn analyze_scenario(
    name: &str,
    avg_file_size: f64,
    args: &Args,
    disk_config: &DiskConfig,
    mem_connections: usize,
) -> ScenarioAnalysis {
    // 计算各资源限制
    // 考虑TCP/IP协议开销(约3%)和JVM Native内存限制
    let network_conn = ((args.net_gbps * 125.0 * 0.97) / (avg_file_size * 1.05)) as usize;
    // 考虑文件系统开销和JVM IO等待
    let disk_conn = ((disk_config.read_speed * 0.75) / (avg_file_size * 1.1)) as usize;
    // 考虑GC暂停时间影响(约15%损耗)
    let cpu_conn = (args.cpu_cores as f64 * (850.0 / avg_file_size.max(1.0))) as usize;
    
    let mut resources = vec![
        ResourceLimit {
            name: "网络带宽".to_string(),
            limiting_factor: false,
            max_connections: network_conn,
            qps: Some(network_conn),
        },
        ResourceLimit {
            name: "磁盘IO".to_string(),
            limiting_factor: false,
            max_connections: disk_conn,
            qps: Some(disk_conn),
        },
        ResourceLimit {
            name: "直接内存".to_string(),
            limiting_factor: false,
            max_connections: mem_connections,
            qps: None,
        },
        ResourceLimit {
            name: "CPU线程".to_string(),
            limiting_factor: false,
            max_connections: cpu_conn,
            qps: Some(cpu_conn * (1000 / avg_file_size.max(1.0) as usize)),
        },
    ];

    // 确定限制因素
    let final_cap = resources.iter()
        .filter(|r| r.qps.is_some())
        .min_by_key(|r| r.max_connections)
        .cloned()
        .unwrap();

    // 标记限制因素
    for r in &mut resources {
        r.limiting_factor = r.max_connections == final_cap.max_connections;
    }

    // 生成关键发现
    let mut key_findings = Vec::new();
    if let Some(limiting_resource) = resources.iter().find(|r| r.limiting_factor) {
        key_findings.push(format!(
            "{}场景({}MB): {}是主要瓶颈 ({} QPS)",
            if avg_file_size > 10.0 { "大文件" } else { "小文件" },
            avg_file_size,
            limiting_resource.name,
            final_cap.qps.unwrap_or(0)
        ));
    }
    key_findings.push(format!(
        "直接内存配置: {:.1}GB满足{}级并发需求",
        args.total_ram * 0.08, mem_connections
    ));

    ScenarioAnalysis {
        name: name.to_string(),
        avg_file_size,
        resources,
        final_capacity: final_cap,
        key_findings,
    }
}
