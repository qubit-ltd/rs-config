# qubit-config

[![Rust CI](https://github.com/qubit-ltd/rs-config/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-config/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-config/coverage-badge.json)](https://qubit-ltd.github.io/rs-config/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-config.svg?color=blue)](https://crates.io/crates/qubit-config)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

一个功能强大、类型安全的 Rust 配置管理系统，提供灵活的配置管理，支持多种数据类型、变量替换、多值属性，以及可插拔的**配置来源（config source）**（文件、环境变量与组合源）。

[English](README.md) | 简体中文

## 特性

- ✅ **纯泛型 API** - 使用 `get<T>()`、`read(ConfigField<T>)` 和 `set<T>()` 泛型方法，支持完整的类型推断
- ✅ **丰富的数据类型** - 支持 Rust 基础类型，以及由 feature 控制的时间、URL 和任意精度数值类型
- ✅ **多值属性** - 每个配置项可以包含多个值，支持列表操作
- ✅ **显式插值** - `*_interpolated` 读取会解析配置中的 `${var_name}`，并默认允许回退到进程环境变量
- ✅ **类型感知 API** - 泛型目标类型在编译期检查；缺失、格式错误或不兼容的配置数据仍会在运行期通过 `ConfigError` 报告
- ✅ **Serde 集成** - 支持 `Config` wire 序列化与 JSON-like 子树反序列化；富类型的原生转换继续通过 typed read 提供
- ✅ **可扩展** - 基于 trait 的设计，易于支持自定义类型
- ✅ **配置来源（ConfigSource）** - 提供 [`ConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/trait.ConfigSource.html) trait 与多种内置实现：TOML、YAML、Java 风格 `.properties`、`.env` 文件、进程环境变量（可选前缀与键名规范化），以及按顺序合并多个来源的 [`CompositeConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.CompositeConfigSource.html)（后加载的来源覆盖同名键）；内置来源按事务语义加载，会校验有歧义的规范化 key，并拒绝 TOML/YAML 单文档内展平后的重复 key
- ✅ **只读访问（ConfigReader）** - 封闭的 [`ConfigReader`](https://docs.rs/qubit-config/latest/qubit_config/trait.ConfigReader.html) trait 为 [`Config`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html) 与 [`ConfigSection`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigSection.html) 提供泛型、多 key 和字段声明读取
- ✅ **可配置解析** - [`ReadOptions`](https://docs.rs/qubit-config/latest/qubit_config/options/struct.ReadOptions.html) 可在全局或单个字段上控制字符串 trim、空白值处理、布尔字面量和标量字符串拆分列表
- ✅ **严格 section（ConfigSection）** - [`Config::section`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.section) 返回严格相对键视图；可通过 [`ConfigSection::section`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigSection.html#method.section) 继续嵌套
- ✅ **安全诊断** - `Debug` 输出保留配置项元数据，并通过 `qubit-redact` 遮盖所有存储值
- ✅ **结构化错误** - [`ConfigError::kind`](https://docs.rs/qubit-config/latest/qubit_config/enum.ConfigError.html#method.kind) 与 [`ConfigError::path`](https://docs.rs/qubit-config/latest/qubit_config/enum.ConfigError.html#method.path) 提供稳定的机器可读上下文，下游无需穷举错误变体
- ✅ **高效核心表示** - 核心值使用枚举表示，并通过 staged source loading 控制合并成本；可插拔 source 在需要动态组合时仍可使用 trait object

## 安装

在您的 `Cargo.toml` 中添加：

```toml
[dependencies]
qubit-config = "0.14"
```

默认 feature 集为空，核心配置读取不会引入可选格式或富类型依赖。需要全部可选能力时启用 `full`：

```toml
qubit-config = { version = "0.14", features = ["full"] }
```

也可以只启用实际需要的能力：

```toml
qubit-config = { version = "0.14", features = ["toml"] }
```

可用 feature flags：

| Feature | 启用内容 |
|---------|----------|
| `bigdecimal` | `BigDecimal` 值及直接 `FromConfig` 支持 |
| `chrono` | Chrono 日期时间值及直接 `FromConfig` 支持 |
| `num-bigint` | `BigInt` 值及直接 `FromConfig` 支持 |
| `url` | URL 值及直接 `FromConfig` 支持 |
| `env-file` | `EnvFileConfigSource` 与 `Config::from_env_file` |
| `toml` | `TomlConfigSource` 与 `Config::from_toml_file` |
| `yaml` | `YamlConfigSource` 与 `Config::from_yaml_file` |
| `rich-types` | 上述四个富类型 feature |
| `formats` | `env-file`、`toml` 与 `yaml` |
| `full` | `rich-types` 与 `formats` |

## 快速开始

```rust
use qubit_config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();

    // 设置配置值
    config.set("port", 8080)?;
    config.set("host", "localhost")?;
    config.set("debug", true)?;

    // 读取配置值（类型推断）
    let port: i32 = config.get("port")?;
    let host: String = config.get("host")?;
    let debug: bool = config.get("debug")?;

    // 使用 turbofish 语法
    let port = config.get::<i32>("port")?;

    // 使用默认值
    let timeout: u64 = config.get_or("timeout", 30)?;

    println!("服务器运行在 {}:{}", host, port);
    Ok(())
}
```

## 核心概念

### Config（配置管理器）

`Config` 结构体是中心配置管理器，存储和管理所有配置属性。

```rust
let mut config = Config::new();
config.set("database.host", "localhost")?;
config.set("database.port", 5432)?;
```

### Property（配置属性）

每个配置项由一个 `Property` 表示，包含：
- 名称（键）
- 保留单值或集合形状的值容器
- 可选描述
- final 标志（设置后不可变）

### ValueContainer（值容器）

一个类型安全的容器，用于保留配置源提供的是单值还是显式集合。单个字符串可按集合转换规则拆分；显式集合中的每个元素只会独立转换，不会再次拆分。

### ConfigReader（只读接口）

[`ConfigReader`](https://docs.rs/qubit-config/latest/qubit_config/trait.ConfigReader.html) 是配置的只读抽象。仅需读取配置时，函数或类型可以接受 `&impl ConfigReader`（或泛型 `R: ConfigReader`），而不必暴露完整的 `&Config`；同一套 API 可用于完整 [`Config`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html) 和 [`ConfigSection`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigSection.html)。`ConfigReader` 包含泛型类型读取方法，因此不是 object-safe，不能用作 `dyn ConfigReader`。

主要读取 API 如下：

| API | 行为 |
|-----|------|
| `get<T>(name)` | 通过 `FromConfig` 读取必填值，不做插值。 |
| `get_optional<T>(name)` | key 不存在或被视为缺失时返回 `Ok(None)`。 |
| `get_or<T>(name, default)` | 仅在 key 不存在或被视为缺失时使用默认值。 |
| `get_any<T>(&[names])` | 按顺序读取第一个未被视为缺失的 key。 |
| `get_optional_any<T>(&[names])` | 多 key 可选读取。 |
| `get_any_or<T>(&[names], default)` | 多 key 默认值读取。 |
| `get_interpolated<T>` / `get_optional_interpolated<T>` / `get_interpolated_or<T>` | 转换前显式执行单 key 插值。 |
| `get_any_interpolated<T>` / `get_optional_any_interpolated<T>` / `get_any_interpolated_or<T>` | 转换前显式执行多 key 插值。 |
| `read(ConfigField<T>)` | 通过字段声明读取，支持 name、alias、default 和字段级解析选项。 |
| `read_interpolated(ConfigField<T>)` / `read_optional_interpolated(ConfigField<T>)` | 显式执行字段插值。 |
| `get_strict` / `get_list_strict` | 精确存储类型读取，不做跨类型转换。 |

默认值不会隐藏错误配置。如果 key 存在，但值解析或类型转换失败，会直接返回错误，不会回退到默认值，也不会继续尝试后面的 alias；显式插值读取同样会直接返回插值错误。

未设置的 property 会被视为缺失；启用相应字符串策略后，标量字符串也可能被视为缺失。
具体的空集合仍是已设置值：`get_optional_list` 返回 `Some(Vec::new())`，且不会使用
默认值。

```rust
use qubit_config::{Config, ConfigError};

let mut config = Config::new();
config.set("worker.threads", "abc")?;

let missing = config.get_or("missing.threads", 4u16)?;
assert_eq!(missing, 4);

let invalid = config.get_or("worker.threads", 4u16);
assert!(matches!(invalid, Err(ConfigError::ConversionError { .. })));
```

`get_or`、`get_any_or`、`get_interpolated_or` 和 `get_any_interpolated_or` 等带 default value 的读取接口支持方便的默认值传法。标量默认值直接使用目标类型；字符串默认值可以直接传 `&str`；字符串列表默认值可以使用数组、切片或借用的 `Vec<String>`。

```rust
let host = config.get_or::<String>("server.host", "localhost")?;
let paths = config.get_or::<Vec<String>>("server.paths", ["bin", "lib"])?;

let paths = config.get_any_or::<Vec<String>>(
    ["server.paths", "SERVER_PATHS"],
    ["cache", "tmp"],
)?;
```

### ConfigSection（严格相对视图）

[`ConfigSection`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigSection.html) 是 `Config` 的零拷贝、严格相对视图。通过 [`Config::section`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.section) 创建；所有键都在 section 路径下解析，例如 section `db` 中的键 `host` 对应 `db.host`。恰好位于 `db` 的标量不属于该 section，只有 `db.host` 等后代可见。使用 [`ConfigSection::section`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigSection.html#method.section) 可继续创建嵌套 section。

判断点分 section 是否存在时使用 `contains_section("db")`。只有明确需要原始字符
前缀匹配时才使用 `contains_key_prefix("db")`；后者也会匹配 `db2` 等同名前缀键。

```rust
use qubit_config::{Config, ConfigReader};

let mut config = Config::new();
config.set("db.host", "localhost")?;
config.set("db.port", 5432i32)?;

let db = config.section("db");
let host: String = db.get("host")?;
let port: i32 = db.get("port")?;
```

### ReadOptions（读取解析选项）

`ReadOptions` 控制配置值如何被解析。它可以设置在 `Config` 全局上，也可以附加到单个 `ConfigField<T>` 上。

| 选项组 | 控制内容 |
|--------|----------|
| `StringConversionOptions` | 字符串 trim，以及空白字符串的处理方式：保留、当作缺失、或拒绝。 |
| `BooleanConversionOptions` | 可接受的布尔字面量和大小写敏感性。 |
| `CollectionConversionOptions` | 是否把标量字符串拆成列表、分隔符、元素 trim，以及空元素策略。 |
| `NumericConversionOptions` | 小数转整数、已有数值转浮点、文本转浮点策略，以及数值文本和 `BigInt` 物化上限。 |
| `DurationConversionOptions` | 数值输入单位、文本后缀规则、输出单位与后缀，以及独立的 Duration 舍入策略。 |
| `ReadOptions` 上的插值设置 | 控制环境变量回退，以及显式插值读取的递归深度、展开次数和输出字节数上限。 |

`ReadOptions::env_friendly()` 适合环境变量风格配置：会 trim 字符串，把空白标量字符串当作缺失，布尔值接受 `true/false`、`1/0`、`yes/no`、`on/off`，并在读取 `Vec<T>` 时按逗号拆分标量字符串、跳过空元素。它允许文本转浮点采用 nearest-even 舍入，但小数转整数与已有数值转浮点仍保持精确。

普通读取永远不会插值 `${...}`。显式插值读取先解析配置项，并默认允许回退到环境变量；可以通过 `ReadOptions` 关闭环境回退，也可以调整递归深度、占位符展开次数和输出字节数上限。

```rust
use qubit_config::{Config, options::ReadOptions};

let mut config = Config::new().with_read_options(ReadOptions::env_friendly());
config.set("HTTP_ENABLED", "yes")?;
config.set("HTTP_PORTS", "8080, 8081,,8082")?;

let enabled: bool = config.get("HTTP_ENABLED")?;
let ports: Vec<u16> = config.get("HTTP_PORTS")?;

assert!(enabled);
assert_eq!(ports, vec![8080, 8081, 8082]);
```

也可以用 builder 风格方法构造更严格或更贴合业务的解析选项：

转换策略类型由 `qubit-datatype` 提供。需要定制这些策略的应用应直接依赖该 crate：

```toml
[dependencies]
qubit-config = "0.14"
qubit-datatype = { version = "0.8", default-features = false, features = ["converter"] }
```

```rust
use qubit_config::{Config, options::ReadOptions};
use qubit_datatype::{
    BlankStringPolicy,
    BooleanConversionOptions,
    CollectionConversionOptions,
    EmptyItemPolicy,
    NumericConversionLimits,
    NumericConversionOptions,
    StringConversionOptions,
};

let options = ReadOptions::default()
    .with_numeric_options(
        NumericConversionOptions::strict().with_limits(
            NumericConversionLimits::default().with_max_text_bytes(4096),
        ),
    )
    .with_string_options(
        StringConversionOptions::default()
            .with_trim(true)
            .with_blank_string_policy(BlankStringPolicy::Reject),
    )
    .with_boolean_options(
        BooleanConversionOptions::strict()
            .with_true_literal("enabled")?
            .with_false_literal("disabled")?,
    )
    .with_collection_options(
        CollectionConversionOptions::default()
            .with_split_scalar_strings(true)
            .with_delimiters([',', ';'])
            .with_trim_items(true)
            .with_empty_item_policy(EmptyItemPolicy::Reject),
    );

let mut config = Config::new().with_read_options(options);
config.set("feature", "enabled")?;
config.set("ports", "8080; 8081")?;

let feature: bool = config.get("feature")?;
let ports: Vec<u16> = config.get("ports")?;
```

### ConfigField（字段声明读取）

当一个逻辑配置项有别名、默认值或字段级解析规则时，使用 `ConfigField<T>`。这样迁移 key、旧 key 和环境变量风格 key 都可以留在配置声明里，而不是散落到业务代码中。

```rust
use qubit_config::{Config, field::ConfigField, options::ReadOptions};

let mut config = Config::new();
config.set("MIME_DETECTOR_ENABLE_PRECISE_DETECTION", "yes")?;

let enabled = config.read(
    ConfigField::<bool>::builder()
        .name("mime.enable_precise_detection")
        .alias("MIME_DETECTOR_ENABLE_PRECISE_DETECTION")
        .alias("ANOTHER_MIME_DETECTOR_ENABLE_PRECISE_DETECTION_PROPERTY")
        .default(false)
        .read_options(ReadOptions::env_friendly())
        .build(),
)?;

assert!(enabled);
```

builder 会强制主 key 明确出现：只有调用 `name(...)` 后，才可以调用 `build()` 生成 `ConfigField<T>`。

### 多 Key 读取

当完整的 `ConfigField<T>` 显得过重时，可以使用 `get_any`、`get_optional_any` 和 `get_any_or` 做普通 alias 读取；需要解析占位符时使用对应的 `*_interpolated` 方法。

```rust
use qubit_config::{Config, options::ReadOptions};

let mut config = Config::new().with_read_options(ReadOptions::env_friendly());
config.set("SERVICE_URL", "http://localhost:8080")?;
config.set("SERVER_TIMEOUT", "30")?;

let url = config.get_any::<String>(["service.url", "SERVICE_URL"])?;
let timeout = config.get_any_or(["server.timeout", "SERVER_TIMEOUT"], 10u64)?;
let optional_port = config.get_optional_any::<u16>(["server.port", "SERVER_PORT"])?;
let retries = config.get_any_or(
    ["server.retries", "SERVER_RETRIES"],
    3u8,
)?;

assert_eq!(url, "http://localhost:8080");
assert_eq!(timeout, 30);
assert_eq!(optional_port, None);
assert_eq!(retries, 3);
```

多 key 读取会按顺序扫描 key。不存在或被视为缺失的值会被跳过；具体空集合仍是已设置
值。如果第一个选中的值无效，会直接返回错误，不会继续尝试后面的 key。

### 配置来源（Configuration sources）

[`ConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/trait.ConfigSource.html) 的实现负责把外部设置写入 [`Config`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html)。可调用 [`merge_from_source`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.merge_from_source)，或在持有 `&mut Config` 时对具体来源调用 `load`。如果不需要在加载前定制目标 `Config`，可以直接使用 [`Config::from_toml_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_toml_file)、[`Config::from_yaml_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_yaml_file)、[`Config::from_properties_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_properties_file)、[`Config::from_env_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_env_file)、[`Config::from_env`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_env) 或 [`Config::from_env_prefix`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_env_prefix) 等便捷构造方法。TOML、YAML 与 `.env` 便捷构造方法分别需要启用 `toml`、`yaml` 与 `env-file` feature。

内置来源和 `Config::merge_from_source` 都按事务语义加载：如果解析或合并失败，目标 `Config` 会保留加载前的状态。

会规范化 key 的环境变量 source 会拒绝空 key 或畸形点号路径，例如 `APP_`、`APP__DB`、`APP_DB__HOST`。TOML 和 YAML source 也会拒绝单个文档内展平后的重复 key，例如字面量 `"server.port"` 与嵌套的 `server.port` 发生冲突。

| 类型 | 作用 |
|------|------|
| [`TomlConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.TomlConfigSource.html) | 读取 TOML 文件；嵌套表展平为点号分隔键 |
| [`YamlConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.YamlConfigSource.html) | 读取 YAML 文件；嵌套映射同样展平 |
| [`PropertiesConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.PropertiesConfigSource.html) | Java `.properties` 文件 |
| [`EnvFileConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.EnvFileConfigSource.html) | `.env` 风格文件 |
| [`EnvConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.EnvConfigSource.html) | 进程环境变量；可选前缀过滤与键名规范化（例如 `APP_SERVER_HOST` → `server.host`） |
| [`CompositeConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.CompositeConfigSource.html) | 按顺序组合多个来源；后出现者覆盖同名键（并受 `Property` 的 final 语义约束） |

```rust
use qubit_config::{Config, source::{
    CompositeConfigSource, ConfigSource, EnvConfigSource, TomlConfigSource,
}};

let mut config = Config::new();
let mut composite = CompositeConfigSource::new();
composite
    .add(TomlConfigSource::from_file("config.toml"))
    .add(EnvConfigSource::with_prefix("APP_"));
config.merge_from_source(&composite)?;
```

```rust
use qubit_config::Config;

let config = Config::from_toml_file("config.toml")?;
let env_config = Config::from_env_prefix("APP_")?;
```

## 使用示例

### 基本配置

```rust
use qubit_config::Config;

let mut config = Config::new();

// 设置各种类型
config.set("port", 8080)?;
config.set("host", "localhost")?;
config.set("debug", true)?;
config.set("timeout", 30.5)?;
config.set("is_use_prefix", "0")?;

// 使用类型推断和转换语义获取值
let port: i32 = config.get("port")?;
let host: String = config.get("host")?;
let debug: bool = config.get("debug")?;
let is_use_prefix: bool = config.get("is_use_prefix")?;

// 需要精确存储类型时仍可使用 strict 读取
assert!(config.get_strict::<bool>("is_use_prefix").is_err());
```

### 多值配置

```rust
// 设置多个值
config.set("ports", vec![8080, 8081, 8082])?;

// 获取所有值
let ports: Vec<i32> = config.get_list("ports")?;

// 逐个添加值
config.set("server", "server1")?;
config.add("server", "server2")?;
config.add("server", "server3")?;

let servers: Vec<String> = config.get_list("server")?;
```

### 变量替换

```rust
config.set("host", "localhost")?;
config.set("port", "8080")?;
config.set("url", "http://${host}:${port}/api")?;

// 普通读取保留占位符。
let raw_url: String = config.get("url")?;
assert_eq!(raw_url, "http://${host}:${port}/api");

// 需要插值时显式调用 interpolated 方法，结果仍可转换成任意支持的类型。
let url: String = config.get_interpolated("url")?;
// 结果: "http://localhost:8080/api"

// 显式插值读取默认允许回退到环境变量；若环境变量不是可信来源，可通过
// ReadOptions 关闭环境回退。
std::env::set_var("APP_ENV", "production");
config.set("env", "${APP_ENV}")?;
let env: String = config.get_interpolated("env")?;
// 结果: "production"
```

### 结构化配置

`deserialize()` 暴露由 mapping、sequence、布尔值、字符串、数字和 null 组成的 JSON-like Serde 视图，并默认保留占位符。需要先解析字符串叶节点时使用 `deserialize_interpolated()`。两者都会应用 `ReadOptions` 的转换规则，例如用 `ReadOptions::env_friendly()` 解析数字字符串、布尔别名、逗号分隔的标量字符串列表，并把空白字符串按缺失值处理。

查找与转换失败会保留原始 `ConfigError` 的 kind、叶子 path 与 source。只有目标类型自身触发的纯 Serde 不匹配，才会在请求的 prefix 返回固定脱敏消息的 `DeserializeError`。

当 `prefix` 非空时，`deserialize(prefix)` 使用严格的根选择语义：如果存在精确的 `prefix` 属性，就把该属性作为反序列化根值；否则用 `prefix.*` 子键组成根对象。同时定义 `prefix` 和 `prefix.*` 会返回 key conflict。带点号的键也必须能组成无歧义对象树，例如同一反序列化对象中不能同时存在 `a` 和 `a.b`。

```rust
use qubit_config::{Config, options::ReadOptions};

#[derive(Debug)]
struct DatabaseConfig {
    host: String,
    port: i32,
    username: String,
    password: String,
}

let mut config = Config::new();
config.set("db.host", "localhost")?;
config.set("db.port", 5432)?;
config.set("db.username", "admin")?;
config.set("db.password", "secret")?;

let db_config = DatabaseConfig {
    host: config.get("db.host")?,
    port: config.get("db.port")?,
    username: config.get("db.username")?,
    password: config.get("db.password")?,
};
```

### 可配置对象

```rust
use qubit_config::{Configurable, Configured};

// 使用 Configured 基类
let mut configured = Configured::new();
configured.config_mut().set("port", 3000)?;
configured.update_config(|config| {
    config.set("host", "localhost")?;
    config.set("workers", 4)?;
    Ok(())
})?;

// 自定义可配置对象
struct Application {
    configured: Configured,
}

impl Application {
    fn new() -> Self {
        Self {
            configured: Configured::new(),
        }
    }

    fn config(&self) -> &Config {
        self.configured.config()
    }

    fn config_mut(&mut self) -> &mut Config {
        self.configured.config_mut()
    }
}

let mut app = Application::new();
app.config_mut().set("port", 3000)?;
```

`config_mut()` 提供直接可变访问，不会自动触发 `on_config_changed()`。如果希望一组修改成功后只触发一次回调，请使用 `update_config()`。

## 支持的数据类型

| Rust 类型 | 说明 | 示例 |
|----------|------|------|
| `bool` | 布尔值；字符串读取默认接受 `true` / `false` 和 `1` / `0`；`ReadOptions::env_friendly()` 还接受 `yes` / `no` 和 `on` / `off` | `true`, `false`, `"0"`, `"yes"` |
| `char` | 字符 | `'a'`, `'中'` |
| `i8`, `i16`, `i32`, `i64`, `i128` | 有符号整数 | `42`, `-100` |
| `u8`, `u16`, `u32`, `u64`, `u128` | 无符号整数 | `255`, `1000` |
| `f32`, `f64` | 浮点数 | `3.14`, `2.718` |
| `String` | 字符串 | `"hello"`, `"世界"` |
| `Vec<T>` | 列表值；配合集合读取选项时，可把标量字符串拆成列表元素 | `[1, 2, 3]`, `"a,b,c"` |
| `chrono::NaiveDate` | 日期 | `2025-01-01` |
| `chrono::NaiveTime` | 时间 | `12:30:45` |
| `chrono::NaiveDateTime` | 日期时间 | `2025-01-01 12:30:45` |
| `chrono::DateTime<Utc>` | 带时区的日期时间 | `2025-01-01T12:30:45Z` |

## 扩展自定义类型

要支持业务特定的配置读取，为目标类型实现 `FromConfig`。实现中可以复用内置的 `FromConfig` 解析，再叠加业务校验；调用方仍然使用 `config.get::<T>()`、`config.get_or::<T>()` 或 `config.read(ConfigField::<T>)`，不需要在每个调用点手写 parse 代码。

```rust
use qubit_config::{Config, ConfigError, ConfigResult, Property};
use qubit_config::from::{ConfigParseContext, FromConfig};

#[derive(Debug, Clone, PartialEq)]
struct Port(u16);

impl Port {
    fn new(value: u16) -> Result<Self, String> {
        if value < 1024 {
            Err("端口号必须 >= 1024".to_string())
        } else {
            Ok(Port(value))
        }
    }

    fn value(&self) -> u16 {
        self.0
    }
}

impl FromConfig for Port {
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        let value = u16::from_config(property, ctx)?;
        Port::new(value).map_err(|message| {
            ConfigError::Other(format!("{}: {message}", ctx.key()))
        })
    }
}

let mut config = Config::new();
config.set("port", "8080")?;

let port: Port = config.get("port")?;
let fallback = config.get_or("fallback_port", Port::new(8080).unwrap())?;
```

只有当你还需要直接存储自定义类型，或需要通过 `get_strict` / `get_list_strict` 做精确存储类型读取时，才需要实现更底层的 `qubit_value` trait。

## API 设计哲学

### 为什么选择纯泛型 API？

我们采用纯泛型方案（如 `get<T>()`、`set<T>()`、`get_or<T>()`、`read(ConfigField<T>)`），而不是为每个类型提供专门的方法（如 `get_i32()`、`get_bool()` 等），原因如下：

1. **通用性强** - 泛型方法可以处理任何实现了相应 trait 的类型，包括自定义类型
2. **代码简洁** - 避免大量重复的类型特定方法
3. **易于维护** - 添加新类型只需实现 trait，无需修改 Config 结构体
4. **符合 Rust 惯用法** - 充分利用 Rust 的类型系统和类型推断

### 类型推断的三种方式

```rust
// 1. 变量类型标注（推荐，最清晰）
let port: i32 = config.get("port")?;

// 2. Turbofish 语法（需要时使用）
let port = config.get::<i32>("port")?;

// 3. 从上下文推断（最简洁）
struct Server {
    port: i32,
}
let server = Server {
    port: config.get("port")?,  // 从字段类型推断
};
```

## 错误处理

配置系统使用 `ConfigResult<T>` 类型进行错误处理：

```rust
#[non_exhaustive]
pub enum ConfigError {
    PropertyNotFound(String),           // 配置项不存在
    PropertyHasNoValue(String),         // 配置项没有值
    TypeMismatch { key: String, expected: DataType, actual: DataType }, // 类型不匹配
    ConversionError { // 结构化且不包含原值的转换失败
        key: String,
        source_index: Option<usize>,
        source: DataConversionError,
    },
    ValueError { key: String, source: ValueError },
    SubstitutionError { path: String, message: String },
    SubstitutionDepthExceeded { path: String, max_depth: usize },
    SubstitutionExpansionLimitExceeded { path: String, max_expansions: usize },
    SubstitutionOutputTooLarge { path: String, max_output_bytes: usize },
    SubstitutionCycle { path: String, chain: Vec<String> },
    MergeError(String),                 // 配置合并失败
    PropertyIsFinal(String),            // 配置项是最终的，不能被覆盖
    KeyConflict { path: String, existing: String, incoming: String }, // key 结构有歧义
    IoError(std::io::Error),            // IO 错误
    ParseError(String),                 // 解析错误
    DeserializeError { path: String, message: String, source: Option<Box<ConfigError>> },
    Other(String),                      // 其他错误
}
```

下游应通过 `ConfigError::kind()` 分支，并通过 `ConfigError::path()` 获取可选的
key 上下文。把值层错误转换为配置错误时，必须使用
`ConfigError::from((path, value_error))` 显式提供 key；不再支持无路径转换。

## 性能考虑

- **枚举值存储** - 核心配置值使用枚举表示，便于稳定地存储和转换
- **变量替换优化** - 使用 `OnceLock` 缓存正则表达式，避免重复编译
- **高效存储** - 配置项使用 `HashMap` 存储，查找时间复杂度 O(1)
- **分阶段 source 加载** - 内置 source 在 composite 和 merge 路径中直接写入已 staged 的 `Config`，保留事务语义，同时避免重复整份配置克隆

## 文档

详细的 API 文档请访问 [docs.rs/qubit-config](https://docs.rs/qubit-config)。

## 依赖项

- `qubit-datatype` - 核心工具和数据类型定义
- `qubit-value` - 值处理框架
- `qubit-redact` - 配置诊断的固定标记遮盖工具
- `serde` - 序列化框架
- `regex` - 正则表达式支持
- `chrono`、`url`、`num-bigint`、`bigdecimal` - 各自同名原子 feature 下的可选富类型支持
- `toml` - `toml` feature 下的 TOML 解析
- `serde_norway` - `yaml` feature 下的 YAML 解析
- `dotenvy` - `env-file` feature 下的 `.env` 文件解析

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
