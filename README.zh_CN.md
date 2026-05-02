# qubit-config

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-config.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-config)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-config/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-config?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-config.svg?color=blue)](https://crates.io/crates/qubit-config)
[![Rust](https://img.shields.io/badge/rust-1.94%2B-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Doc](https://img.shields.io/badge/doc-English-blue.svg)](README.md)

一个功能强大、类型安全的 Rust 配置管理系统，提供灵活的配置管理，支持多种数据类型、变量替换、多值属性，以及可插拔的**配置来源（config source）**（文件、环境变量与组合源）。

[English](README.md) | 简体中文

## 特性

- ✅ **纯泛型 API** - 使用 `get<T>()`、`read(ConfigField<T>)` 和 `set<T>()` 泛型方法，支持完整的类型推断
- ✅ **丰富的数据类型** - 支持所有基本类型、时间类型、字符串、字节数组等
- ✅ **多值属性** - 每个配置项可以包含多个值，支持列表操作
- ✅ **变量替换** - 支持 `${var_name}` 形式的变量替换，可从配置或环境变量中获取
- ✅ **类型安全** - 编译期类型检查，避免运行时类型错误
- ✅ **序列化支持** - 完整的 serde 支持，可序列化和反序列化
- ✅ **可扩展** - 基于 trait 的设计，易于支持自定义类型
- ✅ **配置来源（ConfigSource）** - 提供 [`ConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/trait.ConfigSource.html) trait 与多种内置实现：TOML、YAML、Java 风格 `.properties`、`.env` 文件、进程环境变量（可选前缀与键名规范化），以及按顺序合并多个来源的 [`CompositeConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.CompositeConfigSource.html)（后加载的来源覆盖同名键）；通过 [`Config::merge_from_source`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.merge_from_source) 将外部配置载入 `Config`
- ✅ **只读访问（ConfigReader）** - [`ConfigReader`](https://docs.rs/qubit-config/latest/qubit_config/trait.ConfigReader.html) trait 提供无需修改配置的泛型读取；[`Config`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html) 与 [`ConfigPrefixView`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigPrefixView.html) 均实现该 trait，并包含字符串辅助方法、多 key 读取和字段声明读取
- ✅ **可配置解析** - [`ConfigReadOptions`](https://docs.rs/qubit-config/latest/qubit_config/options/struct.ConfigReadOptions.html) 可在全局或单个字段上控制字符串 trim、空白值处理、布尔字面量和标量字符串拆分列表
- ✅ **前缀视图（ConfigPrefixView）** - [`Config::prefix_view`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.prefix_view) 返回绑定逻辑键前缀的 [`ConfigPrefixView`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigPrefixView.html)（相对键解析为 `前缀.键`）；可通过 [`ConfigPrefixView::prefix_view`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigPrefixView.html#method.prefix_view) 嵌套子前缀
- ✅ **零成本抽象** - 使用枚举而非 trait object，避免动态分发开销

## 安装

在您的 `Cargo.toml` 中添加：

```toml
[dependencies]
qubit-config = "0.11"
```

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
- 多值容器
- 可选描述
- final 标志（设置后不可变）

### MultiValues（多值容器）

一个类型安全的容器，可以保存相同数据类型的多个值。

### ConfigReader（只读接口）

[`ConfigReader`](https://docs.rs/qubit-config/latest/qubit_config/trait.ConfigReader.html) 是配置的只读抽象。仅需读取配置时，函数或类型可以接受 `&impl ConfigReader`（或泛型 `R: ConfigReader`），而不必暴露完整的 `&Config`；同一套 API 可用于完整 [`Config`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html) 和 [`ConfigPrefixView`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigPrefixView.html)。`ConfigReader` 包含泛型类型读取方法，因此不是 object-safe，不能用作 `dyn ConfigReader`。

主要读取 API 如下：

| API | 行为 |
|-----|------|
| `get<T>(name)` | 通过 `FromConfig` 读取必填值。 |
| `get_optional<T>(name)` | key 缺失或为空时返回 `Ok(None)`。 |
| `get_or<T>(name, default)` | 仅在 key 缺失或为空时使用默认值。 |
| `get_any<T>(&[names])` | 按顺序读取第一个存在且非空的 key。 |
| `get_optional_any<T>(&[names])` | 多 key 可选读取。 |
| `get_any_or<T>(&[names], default)` | 多 key 默认值读取。 |
| `get_any_or_with<T>(&[names], default, options)` | 使用显式读取选项的多 key 默认值读取。 |
| `get_string`、`get_string_any`、`get_string_any_or` | 带变量替换的字符串读取。 |
| `read(ConfigField<T>)` | 通过字段声明读取，支持 name、alias、default 和字段级解析选项。 |
| `get_strict` / `get_list_strict` | 精确存储类型读取，不做跨类型转换。 |

默认值不会隐藏错误配置。如果 key 存在，但值解析、类型转换或变量替换失败，会直接返回错误，不会回退到默认值，也不会继续尝试后面的 alias。

```rust
use qubit_config::{Config, ConfigError};

let mut config = Config::new();
config.set("worker.threads", "abc")?;

let missing = config.get_or("missing.threads", 4u16)?;
assert_eq!(missing, 4);

let invalid = config.get_or("worker.threads", 4u16);
assert!(matches!(invalid, Err(ConfigError::ConversionError { .. })));
```

`get_or`、`get_any_or`、`get_any_or_with` 等带 default value 的读取接口现在支持更方便的默认值传法。标量默认值仍直接使用目标类型；字符串默认值可以直接传 `&str`；字符串列表默认值可以使用数组、切片或借用的 `Vec<String>`。

```rust
let host = config.get_or::<String>("server.host", "localhost")?;
let paths = config.get_or::<Vec<String>>("server.paths", ["bin", "lib"])?;

let paths = config.get_any_or::<Vec<String>>(
    &["server.paths", "SERVER_PATHS"],
    ["cache", "tmp"],
)?;
```

### ConfigPrefixView（前缀视图）

[`ConfigPrefixView`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigPrefixView.html) 表示对 `Config` 的零拷贝借用，并带有一个逻辑键前缀。通过 [`Config::prefix_view`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.prefix_view) 创建；传入的键名会在该前缀下解析。例如前缀 `db`、键 `host` 对应存储键 `db.host`。使用 [`ConfigPrefixView::prefix_view`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigPrefixView.html#method.prefix_view) 可得到嵌套前缀视图。

```rust
use qubit_config::{Config, ConfigReader};

let mut config = Config::new();
config.set("db.host", "localhost")?;
config.set("db.port", 5432i32)?;

let db = config.prefix_view("db");
let host: String = db.get_string("host")?;
let port: i32 = db.get("port")?;
```

### ConfigReadOptions（读取解析选项）

`ConfigReadOptions` 控制配置值如何被解析。它可以设置在 `Config` 全局上，也可以附加到单个 `ConfigField<T>` 上。

| 选项组 | 控制内容 |
|--------|----------|
| `StringReadOptions` | 字符串 trim，以及空白字符串的处理方式：保留、当作缺失、或拒绝。 |
| `BooleanReadOptions` | 可接受的布尔字面量和大小写敏感性。 |
| `CollectionReadOptions` | 是否把标量字符串拆成列表、分隔符、元素 trim，以及空元素策略。 |

`ConfigReadOptions::env_friendly()` 适合环境变量风格配置：会 trim 字符串，把空白标量字符串当作缺失，布尔值接受 `true/false`、`1/0`、`yes/no`、`on/off`，并在读取 `Vec<T>` 时按逗号拆分标量字符串、跳过空元素。

```rust
use qubit_config::{Config, options::ConfigReadOptions};

let mut config = Config::new().with_read_options(ConfigReadOptions::env_friendly());
config.set("HTTP_ENABLED", "yes")?;
config.set("HTTP_PORTS", "8080, 8081,,8082")?;

let enabled: bool = config.get("HTTP_ENABLED")?;
let ports: Vec<u16> = config.get("HTTP_PORTS")?;

assert!(enabled);
assert_eq!(ports, vec![8080, 8081, 8082]);
```

也可以用 builder 风格方法构造更严格或更贴合业务的解析选项：

```rust
use qubit_config::{
    Config,
    options::{
        BooleanReadOptions, CollectionReadOptions, ConfigReadOptions, EmptyItemPolicy,
    },
};

let options = ConfigReadOptions::default()
    .with_boolean_options(
        BooleanReadOptions::strict()
            .with_true_literal("enabled")
            .with_false_literal("disabled"),
    )
    .with_collection_options(
        CollectionReadOptions::default()
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
use qubit_config::{Config, field::ConfigField, options::ConfigReadOptions};

let mut config = Config::new();
config.set("MIME_DETECTOR_ENABLE_PRECISE_DETECTION", "yes")?;

let enabled = config.read(
    ConfigField::<bool>::builder()
        .name("mime.enable_precise_detection")
        .alias("MIME_DETECTOR_ENABLE_PRECISE_DETECTION")
        .alias("ANOTHER_MIME_DETECTOR_ENABLE_PRECISE_DETECTION_PROPERTY")
        .default(false)
        .read_options(ConfigReadOptions::env_friendly())
        .build(),
)?;

assert!(enabled);
```

builder 会强制主 key 明确出现：只有调用 `name(...)` 后，才可以调用 `build()` 生成 `ConfigField<T>`。

### 多 Key 读取

当完整的 `ConfigField<T>` 显得过重时，可以使用 `get_any`、`get_optional_any`、`get_any_or` 和 `get_any_or_with` 做轻量 alias 读取。

```rust
use qubit_config::{Config, options::ConfigReadOptions};

let mut config = Config::new().with_read_options(ConfigReadOptions::env_friendly());
config.set("SERVICE_URL", "http://localhost:8080")?;
config.set("SERVER_TIMEOUT", "30")?;

let url = config.get_string_any(&["service.url", "SERVICE_URL"])?;
let timeout = config.get_any_or(&["server.timeout", "SERVER_TIMEOUT"], 10u64)?;
let optional_port = config.get_optional_any::<u16>(&["server.port", "SERVER_PORT"])?;
let retries = config.get_any_or_with(
    &["server.retries", "SERVER_RETRIES"],
    3u8,
    &ConfigReadOptions::env_friendly(),
)?;

assert_eq!(url, "http://localhost:8080");
assert_eq!(timeout, 30);
assert_eq!(optional_port, None);
assert_eq!(retries, 3);
```

多 key 读取会按顺序扫描 key。缺失和空值会被跳过；第一个存在且非空的值会被解析。如果这个值无效，会直接返回错误，不会继续尝试后面的 key。

### 配置来源（Configuration sources）

[`ConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/trait.ConfigSource.html) 的实现负责把外部设置写入 [`Config`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html)。可调用 [`merge_from_source`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.merge_from_source)，或在持有 `&mut Config` 时对具体来源调用 `load`。如果不需要在加载前定制目标 `Config`，可以直接使用 [`Config::from_toml_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_toml_file)、[`Config::from_yaml_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_yaml_file)、[`Config::from_properties_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_properties_file)、[`Config::from_env_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_env_file)、[`Config::from_env`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_env) 或 [`Config::from_env_prefix`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_env_prefix) 等便捷构造方法。

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

// 变量会自动替换
let url = config.get_string("url")?;
// 结果: "http://localhost:8080/api"

// 也支持环境变量
std::env::set_var("APP_ENV", "production");
config.set("env", "${APP_ENV}")?;
let env = config.get_string("env")?;
// 结果: "production"
```

### 结构化配置

```rust
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

## 支持的数据类型

| Rust 类型 | 说明 | 示例 |
|----------|------|------|
| `bool` | 布尔值；字符串读取默认接受 `true` / `false` 和 `1` / `0`；`ConfigReadOptions::env_friendly()` 还接受 `yes` / `no` 和 `on` / `off` | `true`, `false`, `"0"`, `"yes"` |
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
        Port::new(value).map_err(|message| ConfigError::ConversionError {
            key: ctx.key().to_string(),
            message,
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
pub enum ConfigError {
    PropertyNotFound(String),           // 配置项不存在
    PropertyHasNoValue(String),         // 配置项没有值
    TypeMismatch { key: String, expected: DataType, actual: DataType }, // 类型不匹配
    ConversionError { key: String, message: String }, // 类型转换失败
    IndexOutOfBounds { index: usize, len: usize }, // 索引越界
    SubstitutionError(String),          // 变量替换失败
    SubstitutionDepthExceeded(usize),   // 变量替换深度超限
    MergeError(String),                 // 配置合并失败
    PropertyIsFinal(String),            // 配置项是最终的，不能被覆盖
    IoError(std::io::Error),            // IO 错误
    ParseError(String),                 // 解析错误
    Other(String),                      // 其他错误
}
```

## 性能考虑

- **零成本抽象** - 使用枚举而非 trait object，避免动态分发开销
- **变量替换优化** - 使用 `OnceLock` 缓存正则表达式，避免重复编译
- **高效存储** - 配置项使用 `HashMap` 存储，查找时间复杂度 O(1)
- **浅拷贝优化** - 克隆使用浅拷贝（`Arc` 包装时）

## 测试

运行测试套件：

```bash
cargo test
```

运行代码覆盖率测试：

```bash
./coverage.sh
```

## 文档

详细的 API 文档请访问 [docs.rs/qubit-config](https://docs.rs/qubit-config)。

内部设计文档请参阅 [src/README.md](src/README.md)。

## 依赖项

- `qubit-datatype` - 核心工具和数据类型定义
- `qubit-value` - 值处理框架
- `serde` - 序列化框架
- `chrono` - 日期和时间处理
- `regex` - 正则表达式支持
- `toml` - 解析 TOML，供 `TomlConfigSource` 使用
- `serde_yaml` - 解析 YAML，供 `YamlConfigSource` 使用
- `dotenvy` - 解析 `.env` 文件，供 `EnvFileConfigSource` 使用

## 发展路线图

- [ ] 更多配置格式加载器（如 JSON、XML）
- [ ] 除有序 `CompositeConfigSource` 外的高级合并 / 覆盖策略
- [ ] 支持配置监听和热重载
- [ ] 支持配置验证框架
- [ ] 支持配置加密
- [ ] 提供线程安全的包装类型 `SyncConfig`

## 贡献

欢迎贡献！请随时提交 Pull Request。

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu, Qubit Co. Ltd. All rights reserved.

根据 Apache 许可证 2.0 版（"许可证"）授权；
除非遵守许可证，否则您不得使用此文件。
您可以在以下位置获取许可证副本：

    http://www.apache.org/licenses/LICENSE-2.0

除非适用法律要求或书面同意，否则根据许可证分发的软件
按"原样"分发，不附带任何明示或暗示的担保或条件。
有关许可证下的特定语言管理权限和限制，请参阅许可证。

完整的许可证文本请参阅 [LICENSE](LICENSE)。

## 作者

**胡海星** - *Qubit Co. Ltd.*

---

有关 Qubit Rust 库的更多信息，请访问我们的 [GitHub 组织](https://github.com/qubit-ltd)。
