# qubit-config

[![Rust CI](https://github.com/qubit-ltd/rs-config/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-config/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-config/coverage-badge.json)](https://qubit-ltd.github.io/rs-config/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-config.svg?color=blue)](https://crates.io/crates/qubit-config)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

`qubit-config` 是一个面向 Rust 应用的类型安全配置库，适合需要组合默认值、配置文件和环境变量、又不希望在业务代码中到处编写字符串解析逻辑的场景。它提供明确的泛型读取 API，并保留配置来源层、类型转换、插值和错误上下文。

## 安装

```toml
[dependencies]
qubit-config = "0.16"
```

默认 feature 集为空，因此核心 API 不会启用可选文件格式或富类型。应用可以按需启用 feature，也可以使用 `full` 开启完整的可选能力：

```toml
qubit-config = { version = "0.16", features = ["toml", "env-file"] }
```

或者启用完整的可选能力：

```toml
qubit-config = { version = "0.16", features = ["full"] }
```

| Feature | 提供能力 |
| --- | --- |
| `bigdecimal` | `BigDecimal` 值及其转换支持 |
| `chrono` | Chrono 日期时间值及其转换支持 |
| `num-bigint` | `BigInt` 值及其转换支持 |
| `url` | URL 值及其转换支持 |
| `env-file` | 通过 `EnvFileConfigSource` 和 `Config::from_env_file` 加载 `.env` |
| `toml` | 通过 `TomlConfigSource` 和 `Config::from_toml_file` 加载 TOML |
| `yaml` | 通过 `YamlConfigSource` 和 `Config::from_yaml_file` 加载 YAML |
| `rich-types` | `bigdecimal`、`chrono`、`num-bigint` 和 `url` |
| `formats` | `env-file`、`toml` 和 `yaml` |
| `full` | `rich-types` 和 `formats` |

## 快速开始

核心工作流是使用可变的 `Config` 保存配置，再通过类型化 API 读取。相同的泛型接口可以读取基础类型、集合和实现了 `FromConfig` 的类型。

```rust
use qubit_config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.set("server.host", "localhost")?;
    config.set("server.port", 8080)?;
    config.set("server.debug", true)?;

    let host: String = config.get("server.host")?;
    let port: u16 = config.get("server.port")?;
    let timeout: u64 = config.get_or("server.timeout", 30)?;

    assert_eq!(host, "localhost");
    assert_eq!(port, 8080);
    assert_eq!(timeout, 30);
    Ok(())
}
```

## 一个真实的配置场景

应用可以先加载提交到仓库的基础配置，再叠加优先级更高的环境变量层。来源按照添加顺序应用；对于同一个 key，后加载的来源会覆盖前一个来源，除非已有 property 被标记为 final。

```rust
use qubit_config::{Config, ConfigReader};
use qubit_config::source::{
    CompositeConfigSource, EnvConfigSource, PropertiesConfigSource,
};

fn load_server_config() -> Result<(String, u16), Box<dyn std::error::Error>> {
    let mut sources = CompositeConfigSource::new();
    sources.add(PropertiesConfigSource::from_content(
        "server.host=localhost\nserver.port=8080\n",
    ));
    sources.add(EnvConfigSource::with_prefix("APP_"));

    let mut config = Config::new();
    config.merge_properties_from_source(&sources)?;
    let server = config.section("server")?;

    Ok((server.get("host")?, server.get("port")?))
}
```

设置 `APP_SERVER__HOST` 和 `APP_SERVER__PORT` 后，环境变量层会在去除前缀、转小写并把双下划线转换为点号后提供最终值；单个下划线保留在层级片段中。如果规范化使不同的环境变量名称映射到同一个 key，加载会返回 `ConfigError::KeyConflict`，并按字典序报告冲突名称；不会根据进程环境变量的遍历顺序静默选择其中一个。启用对应 feature 后，相同的组合方式也可以使用 `TomlConfigSource`、`YamlConfigSource` 或 `EnvFileConfigSource`。

## 结构化读取与自定义策略

当一个 subtree 自然对应某个 Serde 类型时，可以使用 `Config::deserialize`：

```rust
use qubit_config::Config;
use qubit_config::ReadPolicy;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionOperationLimits;
use serde::Deserialize;

#[derive(Deserialize)]
struct Database {
    host: String,
    port: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let operation = ConversionOperationLimits::default()
        .with_max_input_bytes(128);
    let conversion = ConversionLimits::default()
        .with_operation_limits(operation);
    let policy = ReadPolicy::default().with_conversion_limits(conversion);
    let mut config = Config::new().with_default_read_policy(policy);
    config.set("db.host", "localhost")?;
    config.set("db.port", "5432")?;
    let db = config.deserialize::<Database>("db")?;
    assert_eq!(db.host, "localhost");
    assert_eq!(db.port, 5432);
    Ok(())
}
```

一次 `ConfigSerdeExt::deserialize::<T>` 只创建一个 `ConversionSession`，并让该会话
贯穿本次物化中的所有字段、嵌套 map、sequence、enum 和 variant。因此，上面的
operation limits 会在两个字段之间累计。彼此独立的普通 `get` 调用各自创建新 session，
不会共享消耗。某个字段转换失败时，本次物化此前已接受的消耗不会回滚；被拒绝的
charge 本身仍保持原子性。

结构化读取默认拒绝目标类型未声明的配置字段，并通过
`ConfigError::UnknownProperties` 返回 root-relative 路径。请使用目标类型的
Serde 结构（`rename`、`alias`、`default`、嵌套类型、map 或 `flatten`）声明可接受
字段；只有明确允许开放字段时，才使用 `deserialize_lenient` 或
`deserialize_interpolated_lenient`。

使用结构化或 JSON 示例时，请直接添加 Serde 依赖：

```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

如果需要直接定制转换选项，请依赖其所属的 `qubit-datatype` crate：

```toml
qubit-datatype = { version = "0.11", default-features = false, features = ["converter"] }
```

## 为什么需要这个项目

配置通常以字符串形式到达，但应用代码需要类型化值、默认值、列表、嵌套 section 和有用的失败上下文。`qubit-config` 把这些职责集中在一个库中：

- 来源会生成独立的配置 layer，可以先检查，也可以事务式合并。
- `ConfigReader` 为 `Config` 和 `ConfigSection` 提供类型化、可选、带默认值、多 key、列表和严格读取。
- 通过 `ReadPolicy` 明确控制转换规则；`read_with` 可以临时使用借用的 policy。
- 插值通过 `*_interpolated` 方法显式开启；回退到环境变量必须显式配置 `InterpolationSources::ConfigThenEnv`。
- `ConfigError::kind()`、`path()`、`source_id()` 和 `candidate_paths()` 提供稳定的诊断上下文，无需穷举错误变体。

## 提供什么，以及不提供什么

当前建议作为稳定核心的 API 是 `Config`、`ConfigReader`、`ConfigSection`、
`ReadPolicy` 和 `ConfigSerdeExt`。source adapter、持久化与 wire 解码以及低层
`Property` 操作属于应用确有需要时使用的独立层。本库提供泛型类型转换、多值
属性、严格相对 section、来源组合、可选的 TOML/YAML/`.env` 加载器、JSON 持久化
解码和脱敏的 `Debug` 输出。
持久化层分别使用 `JsonDecodeLimits`/`JsonDecodeSession` 和
`JsonEncodeLimits`/`JsonEncodeSession`，因此输入准入与输出限制不会消耗错误方向的
字节资源。`Config` 的普通 `Deserialize` 会应用默认的已解码结构、payload、
property 数量和 property key 限制；通用 Serde deserializer 无法看到原始字节或
JSON 词法 token，因此不可信 JSON 还必须通过 `Config::decode_json_slice` 执行
原始输入准入。

所有内置 source 都通过 `SourceLoadContext` 和 crate-owned executor 加载。自定义
`ConfigSource` 必须通过 context（例如 `context.set(...)`）写入输出，并在执行外部
I/O 或 parser 工作前主动报告输入字节和解析节点；最终 layer 无法反推出未报告的
工作量。通过 context 的输入字节、assignment、解析节点、子 source 数量和嵌套深度
会同时检查 source 局部预算与 composite 聚合预算，任一预算拒绝时不会只扣减其中一层。
TOML 和 YAML 在 parser 边界存在明确例外：第三方 parser
会先物化完整 AST，节点、assignment 和深度只能在 flatten 阶段记账，所以这些限制
不能约束 parser 自身的内存分配或递归。要获得该保证，未来需要流式 parser。只有明确
了解输入边界时才应放宽 `SourceLimits`。

本库不会在普通读取时静默执行插值，不会在加载 `.env` 文件时展开进程环境占位符，不会用默认值掩盖已存在但无效的值，也不会把 `ConfigReader` 变成 `dyn` trait object：它的泛型方法使其不满足 object-safe。路径规则、source 失败行为、结构化反序列化、自定义转换和排障细节请参阅用户手册。

## 延伸阅读

- [English user guide](doc/user_guide.md)
- [中文用户手册](doc/user_guide.zh_CN.md)
- [docs.rs API 文档](https://docs.rs/qubit-config)
- [English README](README.md)
- [仓库](https://github.com/qubit-ltd/rs-config)

## 测试

```bash
# 使用默认 feature 集运行测试
cargo test

# 使用项目声明的全部 feature 运行测试
cargo test --all-features

# 运行项目 CI 检查
./ci-check.sh

# 检查代码覆盖率
./coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./align-ci.sh`格式化代码，运行`./ci-check.sh`对齐CI要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-config](https://github.com/qubit-ltd/rs-config)
