# 文件传输系统安全分析工具 (sa)

一个用于分析文件上传下载Java构建的系统性能与安全性的Rust命令行工具，提供内存使用评估、风险分析和优化建议。

## 功能特性

- 📊 系统配置分析 - 评估服务器硬件配置
- 🛡️ 安全性分析 - 计算内存安全系数和风险等级
- 🔄 场景模拟 - 模拟正常/突发/大文件/高并发场景
- ⚙️ JVM配置建议 - 生成针对性的JVM调优参数
- 🎨 彩色终端输出 - 直观显示分析结果
- 📝 Markdown报告 - 支持生成详细分析报告

## 安装

### 从源码构建

```bash
# 安装Rust (https://rustup.rs/)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆项目
git clone https://github.com/your-repo/sa.git
cd sa

# 构建发布版本
make release

# 安装到系统路径 (需要管理员权限)
sudo make install
```

### 从预编译二进制安装

下载对应平台的二进制文件，然后：

```bash
chmod +x sa
sudo mv sa /usr/local/bin/
```

## 使用方法

```bash
sa [OPTIONS]
```

### 主要选项

| 选项                         | 描述                                | 默认值   | 示例值 |
| ---------------------------- | ----------------------------------- | -------- | ------ |
| `-r, --total-ram`            | 服务器总内存(GB)                    | 32       | 64     |
| `-c, --cpu-cores`            | CPU核心数                           | 16       | 32     |
| `-w, --net-gbps`             | 网络带宽(Gbps)                      | 1        | 10     |
| `-d, --disk-type`            | 磁盘类型 [sata_hdd, sata_ssd, nvme] | sata_ssd | nvme   |
| `-f, --avg-file-size`        | 平均文件大小(MB)                    | 10       | 50     |
| `-n, --expected-connections` | 预期最大并发连接数                  | 1000     | 5000   |
| `-b, --burst-factor`         | 最大突发流量倍数                    | 3        | 5      |
| `-p, --enable-memory-guard`  | 是否启用内存防护                    | true     | false  |
| `-m, --enable-memory-mapping`| 是否启用内存映射文件优化            | false    | true   |
| `-l, --complexity`           | 应用复杂度级别 [low, medium, high]  | medium   | high   |
| `-g, --generate-markdown`    | 生成Markdown报告                    | false    | true   |

### 示例

1. 分析一个32GB内存、16核CPU、1Gbps网络、使用NVMe磁盘的系统：

```bash
sa --total-ram 32 --cpu-cores 16 --net-gbps 1 --disk-type nvme
```

2. 分析大文件处理场景并生成报告：

```bash
sa --total-ram 64 --cpu-cores 32 --net-gbps 10 --disk-type nvme \
   --avg-file-size 100 --expected-connections 2000 --complexity high \
   --enable-memory-mapping --generate-markdown
```

3. 快速测试小文件高并发场景：

```bash
sa --total-ram 16 --cpu-cores 8 --avg-file-size 5 --expected-connections 5000
```

## 输出报告

工具会生成六部分分析报告：

1. **系统配置** - 显示输入参数和计算出的推荐值
2. **系统极限评估** - 基于6-12个月稳定运行的理论极限
3. **负载场景模拟** - 五种典型场景下的内存使用情况
4. **内存安全分析** - 显示内存安全系数和风险等级
5. **全链路性能分析** - 各资源瓶颈和QPS评估
6. **JVM配置建议** - 针对性的调优参数和版本兼容性矩阵

## 报告示例

![示例报告截图](https://example.com/sa-report-screenshot.png)

生成Markdown报告示例：

```bash
sa --generate-markdown
```

报告将保存为`sa_report.md`

## 跨平台构建

在macOS上构建Windows可执行文件：

```bash
# 安装cross工具
cargo install cross

# 构建Windows版本
make release-win
```

构建结果会生成在`target/x86_64-pc-windows-gnu/release/sa.exe`

## 开发

### 构建和运行

```bash
# 调试构建
make build

# 运行
make run

# 运行测试
make test
```

### 代码质量检查

```bash
# 格式化代码
make fmt

# Lint检查
make lint
```

## 贡献指南

欢迎提交Issue和Pull Request。请确保：

1. 代码通过`make fmt`和`make lint`
2. 添加/更新相关测试
3. 更新文档(README.md)
4. 遵循现有代码风格

## 性能优化建议

对于大型分析场景，可以：

1. 使用`--release`构建以获得最佳性能
2. 对于超过10,000连接的分析，增加JVM堆内存：
   ```bash
   RUSTFLAGS="-C target-cpu=native" make release
   ```

## 常见问题

**Q: 如何分析超大文件(>1GB)场景?**  
A: 使用`--enable-memory-mapping`选项并增加直接内存：
```bash
sa --avg-file-size 1024 --enable-memory-mapping --total-ram 128
```

**Q: 报告中的危险警告是什么意思?**  
A: 表示内存使用超过85%，需要调整配置或优化应用

## 许可证

MIT License
