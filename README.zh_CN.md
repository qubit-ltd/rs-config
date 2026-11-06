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

<!-- example: config_quickstart -->
```rust
//! Shows basic configuration writes and typed reads with a default value.

use qubit_config::Config;

/// Reads required server fields and supplies a fallback timeout.
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

<!-- example: config_sources -->
```rust
//! Demonstrates merging property and environment configuration sources.

use qubit_config::Config;
use qubit_config::ConfigReader;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::EnvConfigSource;
use qubit_config::source::PropertiesConfigSource;

/// Loads and prints a server endpoint from layered configuration sources.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (host, port) = load_server_config()?;
    println!("server listening at {host}:{port}");
    Ok(())
}

/// Merges property defaults with environment overrides and reads server fields.
fn load_server_config() -> Result<(String, u16), Box<dyn std::error::Error>> {
    let mut sources = CompositeConfigSource::new();
    sources.add(PropertiesConfigSource::from_content(
        "server.host=localhost\nserver.port=8080\n",
    ));
    sources.add(EnvConfigSource::from_prefix("APP_"));

    let mut config = Config::new();
    config.merge_properties_from_source(&sources)?;
    let server = config.section("server")?;

    Ok((server.get("host")?, server.get("port")?))
}
```

设置 `APP_SERVER__HOST` 和 `APP_SERVER__PORT` 后，环境变量层会在去除前缀、转小写并把双下划线转换为点号后提供最终值；单个下划线保留在层级片段中。如果规范化使不同的环境变量名称映射到同一个 key，加载会返回 `ConfigError::KeyConflict`，并按字典序报告冲突名称；不会根据进程环境变量的遍历顺序静默选择其中一个。启用对应 feature 后，相同的组合方式也可以使用 `TomlConfigSource`、`YamlConfigSource` 或 `EnvFileConfigSource`。

Properties、TOML、YAML、`.env` 和 composite 来源的 builder 可通过可选的 `label` 为诊断设置稳定名称；`EnvConfigSource` 保留固定的 `process environment` 标识。标签只属于诊断元数据，不得包含秘密，也不会改变来源顺序或 Wire V1。嵌套预算归因和恢复默认标签的说明见[用户手册](doc/user_guide.zh_CN.md#加载和合并配置源)。

## 结构化读取与自定义策略

当一个 subtree 自然对应某个 Serde 类型时，可以使用 `Config::deserialize`：

<!-- example: config_structured -->
```rust
//! Demonstrates bounded structured reads and unknown-field policy choices.

use qubit_config::Config;
use qubit_config::ConfigDeserializeOptions;
use qubit_config::ConfigError;
use qubit_config::ReadPolicy;
use qubit_config::UnknownFieldPolicy;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionOperationLimits;
use serde::Deserialize;

/// Fields selected from the database section in this structured-read example.
#[derive(Debug, Deserialize, PartialEq)]
struct Database {
    /// Database host name.
    host: String,
    /// Database service port.
    port: u16,
}

/// Compares strict and lenient reads under a conversion input budget.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let operation = ConversionOperationLimits::builder().max_input_bytes(128).build();
    let conversion = ConversionLimits::builder().operation_limits(operation).build();
    let policy = ReadPolicy::builder().conversion_limits(conversion).build();
    let mut config = Config::builder().default_read_policy(policy).build();
    config.set("db.host", "localhost")?;
    config.set("db.port", "5432")?;
    config.set("db.pool_size", 16)?;

    let error = config
        .deserialize::<Database>("db")
        .expect_err("strict reads reject unknown fields");
    assert!(matches!(error, ConfigError::UnknownProperties { .. }));

    let database = config.deserialize_with::<Database>(
        "db",
        ConfigDeserializeOptions {
            unknown_fields: UnknownFieldPolicy::Ignore,
            ..Default::default()
        },
    )?;
    assert_eq!(
        database,
        Database {
            host: "localhost".to_owned(),
            port: 5432,
        },
    );
    Ok(())
}
```

一次 `ConfigSerdeExt::deserialize::<T>` 只创建一个 `ConversionSession`，并让该会话
贯穿本次物化中的所有字段、嵌套 map、sequence、enum 和 variant。因此，上面的
operation limits 会在两个字段之间累计。彼此独立的普通 `get` 调用各自创建新 session，
不会共享消耗。某个字段转换失败时，本次物化此前已接受的消耗不会回滚；被拒绝的
charge 本身仍保持原子性。

结构化读取会拒绝源反序列化器收到 ignored-value 请求时暴露的字段，并通过
`ConfigError::UnknownProperties` 返回 root-relative 路径。部分 `flatten`、untagged enum
和自定义 `Deserialize` 实现会缓冲或消费 key，因此源反序列化器不会为每个被丢弃字段收到
ignored-value 请求。
若要求闭合 schema，可在普通 struct 目标（以及 untagged enum 的每个 payload struct）上使用
`#[serde(deny_unknown_fields)]`；这会返回经过脱敏的 Serde 反序列化错误。Serde 不允许
`deny_unknown_fields` 与 `flatten` 组合，因此应改用闭合的嵌套 struct，或在反序列化前按应用
schema 校验 key。只有明确允许开放字段时，才选择 `UnknownFieldPolicy::Ignore`。
`deserialize(prefix)` 使用默认选项：不插值、拒绝未知字段；
`deserialize_with(prefix, options)` 还可以设置 `interpolate: true`。
结构化读取只保留这两个入口，空 prefix 表示整个可见范围。

目标 visitor 执行前，会对选中的完整输入执行准入，包括被忽略的字段。开启插值时，原始输入与
展开后的输入分别使用独立预算检查；随后所有叶子转换共享另一个 `ConversionSession`。
内部借用原生值和集合，返回值仍要求 `DeserializeOwned`。

使用结构化或 JSON 示例时，请直接添加 Serde 依赖：

```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

如果需要直接定制转换选项，请依赖其所属的 `qubit-datatype` crate：

```toml
qubit-datatype = { version = "0.15", default-features = false, features = ["converter"] }
```

## 为什么需要这个项目

配置通常以字符串形式到达，但应用代码需要类型化值、默认值、列表、嵌套 section 和有用的失败上下文。`qubit-config` 把这些职责集中在一个库中：

- 来源会生成独立的配置 layer，可以先检查，也可以事务式合并。
- `ConfigReader` 为 `Config` 和 `ConfigSection` 提供类型化、可选、带默认值、多 key、列表和严格读取。
- 通过 `ReadPolicy` 明确控制转换规则；`read_with` 可以临时使用借用的 policy。
- 插值通过普通读取的 `*_interpolated` 方法或结构化读取选项显式开启；回退到环境变量必须显式配置 `InterpolationSources::ConfigThenEnv`。
- `ConfigError::kind()`、`path()`、`source_id()` 和 `candidate_paths()` 提供稳定的诊断上下文，无需穷举错误变体。
- `ConfigReader::ensure_known_keys` 可显式校验应用 schema；`usize` 可作为类型化读取目标，并会检查当前平台的整数范围。
- 有界 wire 解码使用安全的 JSON Pointer 路径，与点分配置路径含义不同。

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
已知键校验仍须显式调用，`usize` 是读取目标而非存储类型。Wire 表示失败使用 `ConfigWireDecodeError::Representation`；未被有界预检识别的深层失败归类为 `InvalidValue`。
此前归为 `Json(Data)` 的业务表示失败现在归为 `Representation`；语法和预算错误仍保留原有分类。
普通 Serde 解码会按 Wire schema 角色读取原生 Float32/Float64 字段，即使依赖图统一启用
`serde_json/arbitrary_precision` 也一样。任意 JSON 领域数字仍遵循通用 Serde 事件语义；需要保留
JSON 数字语义时应使用专用 JSON helper。

内置配置源通过受控上下文加载，同时执行本地和复合来源的资源限制。自定义来源必须在执行外部工作前
主动报告，因为框架无法从最终配置层推断遗漏的工作量。TOML 和 YAML 会在构建 AST 前执行解析准入检查；
YAML 会拒绝 anchor 和 alias token。`.env` 加载保留变量占位符字面值，final 属性不会被覆盖，库管理的加载入口
会调用 `load_into`。组合来源执行器还固定限制最多 256 条活动子来源边（根深度为 0），与 `max_sources`
独立，`SourceLimits::unbounded()` 也不能关闭该保护。更深的组合应扁平化。此保护只覆盖内置组合调度；
自定义来源自行递归仍由实现负责。来源限制不保证严格的 RSS 或 CPU 上限。详见[来源加载指南](doc/user_guide.zh_CN.md#加载和合并配置源)、
[来源限制指南](doc/user_guide.zh_CN.md#配置源限制)和[设计专题](doc/design.zh_CN.md#资源预算)。

本库不会在普通读取时静默执行插值，不会在加载 `.env` 文件时展开进程环境占位符，也不会用默认值掩盖已存在但无效的值。`ConfigReader` 是封闭（sealed）trait，不能由第三方实现；它包含泛型方法，因此不支持 `dyn ConfigReader`。路径规则、配置源失败行为、结构化反序列化、自定义转换和排障细节请参阅用户手册；当前组件边界与兼容承诺记录在设计说明中。

## 延伸阅读

`Config` 仍实现 `serde::Serialize`，并提供 `encode_json_vec()` 持久化配置 envelope；
`deserialize` 则从配置读取业务类型。本次不增加业务结构体反向写入配置的 serializer，
Wire V1 保持不变。

- [English user guide](doc/user_guide.md)
- [Migrating from qubit-config 0.14](doc/user_guide.md#migrating-from-qubit-config-014)
- [Custom target types](doc/user_guide.md#custom-target-types)
- [中文用户手册](doc/user_guide.zh_CN.md)
- [从 qubit-config 0.14 迁移](doc/user_guide.zh_CN.md#从-qubit-config-014-迁移)
- [自定义目标类型](doc/user_guide.zh_CN.md#custom-target-types)
- [English design document](doc/design.md)
- [中文设计说明](doc/design.zh_CN.md)
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
./.infra/bin/ci-check.sh

# 生成共享覆盖率报告并检查本 crate 的逐文件门槛
./scripts/check-project-coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./.infra/bin/align-ci.sh`格式化代码，运行`./.infra/bin/ci-check.sh`对齐CI要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-config](https://github.com/qubit-ltd/rs-config)
