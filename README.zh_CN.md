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
qubit-config = "0.15"
```

默认 feature 集为空，因此核心 API 不会启用可选文件格式或富类型。应用可以按需启用 feature，也可以使用 `full` 开启完整的可选能力：

```toml
qubit-config = { version = "0.15", features = ["toml", "env-file"] }
```

或者启用完整的可选能力：

```toml
qubit-config = { version = "0.15", features = ["full"] }
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
    config.merge_from_source(&sources)?;
    let server = config.section("server")?;

    Ok((server.get("host")?, server.get("port")?))
}
```

设置 `APP_SERVER_HOST` 和 `APP_SERVER_PORT` 后，环境变量层会在去除前缀、转小写并把下划线转换为点号后提供最终值。启用对应 feature 后，相同的组合方式也可以使用 `TomlConfigSource`、`YamlConfigSource` 或 `EnvFileConfigSource`。

## 结构化读取与自定义策略

当一个 subtree 自然对应某个 Serde 类型时，可以使用 `Config::deserialize`：

```rust
use qubit_config::Config;
use serde::Deserialize;

#[derive(Deserialize)]
struct Database {
    host: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.set("db.host", "localhost")?;
    let db: Database = config.deserialize("db")?;
    assert_eq!(db.host, "localhost");
    Ok(())
}
```

使用结构化或 JSON 示例时，请直接添加 Serde 依赖：

```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

如果需要直接定制转换选项，请依赖其所属的 `qubit-datatype` crate：

```toml
qubit-datatype = { version = "0.10", default-features = false, features = ["converter"] }
```

## 为什么需要这个项目

配置通常以字符串形式到达，但应用代码需要类型化值、默认值、列表、嵌套 section 和有用的失败上下文。`qubit-config` 把这些职责集中在一个库中：

- 来源会生成独立的配置 layer，可以先检查，也可以事务式合并。
- `ConfigReader` 为 `Config` 和 `ConfigSection` 提供类型化、可选、带默认值、多 key、列表和严格读取。
- 通过 `ReadPolicy` 明确控制转换规则；`read_with` 可以临时使用借用的 policy。
- 插值通过 `*_interpolated` 方法显式开启；回退到环境变量必须显式配置 `InterpolationSources::ConfigThenEnv`。
- `ConfigError::kind()`、`path()` 和 `candidate_paths()` 提供稳定的诊断上下文，无需穷举错误变体。

## 提供什么，以及不提供什么

本库提供泛型类型转换、多值属性、严格相对 section、来源组合、可选的 TOML/YAML/`.env` 加载器、JSON 持久化解码和脱敏的 `Debug` 输出。内置文本 source 默认限制输入 8 MiB、assignment 65,536 次、嵌套深度 64；如果输入可信且确实需要更大边界，可以使用 `SourceLimits` 定制。

本库不会在普通读取时静默执行插值，不会用默认值掩盖已存在但无效的值，也不会把 `ConfigReader` 变成 `dyn` trait object：它的泛型方法使其不满足 object-safe。路径规则、source 失败行为、结构化反序列化、自定义转换和排障细节请参阅用户手册。

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
