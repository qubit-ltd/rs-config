# prism3-config

[![CircleCI](https://circleci.com/gh/3-prism/prism3-rust-config.svg?style=shield)](https://circleci.com/gh/3-prism/prism3-rust-config)
[![Coverage Status](https://coveralls.io/repos/github/3-prism/prism3-rust-config/badge.svg?branch=main)](https://coveralls.io/github/3-prism/prism3-rust-config?branch=main)
[![Crates.io](https://img.shields.io/crates/v/prism3-config.svg?color=blue)](https://crates.io/crates/prism3-config)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Doc](https://img.shields.io/badge/doc-English-blue.svg)](README.md)

一个功能强大、类型安全的 Rust 配置管理系统，提供灵活的配置管理，支持多种数据类型、变量替换和多值属性。

[English](README.md) | 简体中文

## 特性

- ✅ **纯泛型 API** - 使用 `get<T>()` 和 `set<T>()` 泛型方法，支持完整的类型推断
- ✅ **丰富的数据类型** - 支持所有基本类型、时间类型、字符串、字节数组等
- ✅ **多值属性** - 每个配置项可以包含多个值，支持列表操作
- ✅ **变量替换** - 支持 `${var_name}` 形式的变量替换，可从配置或环境变量中获取
- ✅ **类型安全** - 编译期类型检查，避免运行时类型错误
- ✅ **序列化支持** - 完整的 serde 支持，可序列化和反序列化
- ✅ **可扩展** - 基于 trait 的设计，易于支持自定义类型
- ✅ **零成本抽象** - 使用枚举而非 trait object，避免动态分发开销

## 安装

在您的 `Cargo.toml` 中添加：

```toml
[dependencies]
prism3-config = "0.1"
```

## 快速开始

```rust
use prism3_config::Config;

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
    let timeout: u64 = config.get_or("timeout", 30);

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

## 使用示例

### 基本配置

```rust
use prism3_config::Config;

let mut config = Config::new();

// 设置各种类型
config.set("port", 8080)?;
config.set("host", "localhost")?;
config.set("debug", true)?;
config.set("timeout", 30.5)?;

// 使用类型推断获取值
let port: i32 = config.get("port")?;
let host: String = config.get("host")?;
let debug: bool = config.get("debug")?;
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
use prism3_config::{Configurable, Configured};

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
| `bool` | 布尔值 | `true`, `false` |
| `char` | 字符 | `'a'`, `'中'` |
| `i8`, `i16`, `i32`, `i64`, `i128` | 有符号整数 | `42`, `-100` |
| `u8`, `u16`, `u32`, `u64`, `u128` | 无符号整数 | `255`, `1000` |
| `f32`, `f64` | 浮点数 | `3.14`, `2.718` |
| `String` | 字符串 | `"hello"`, `"世界"` |
| `Vec<u8>` | 字节数组 | `[1, 2, 3, 4]` |
| `chrono::NaiveDate` | 日期 | `2025-01-01` |
| `chrono::NaiveTime` | 时间 | `12:30:45` |
| `chrono::NaiveDateTime` | 日期时间 | `2025-01-01 12:30:45` |
| `chrono::DateTime<Utc>` | 带时区的日期时间 | `2025-01-01T12:30:45Z` |

## 扩展自定义类型

要在配置系统中支持自定义类型，您需要实现 `prism3_value` 中的必要 trait。配置系统使用 `MultiValues` 基础设施进行类型安全的存储和检索。

以下是如何使用自定义类型的示例：

```rust
use prism3_config::Config;

// 定义自定义类型
#[derive(Debug, Clone, PartialEq)]
struct Port(u16);

// 您可以将配置系统与可以转换为/从基本类型转换的类型一起使用
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

// 在配置系统中使用
let mut config = Config::new();

// 以基本类型存储端口
config.set("port", 8080u16)?;

// 检索并包装在自定义类型中
let port_value: u16 = config.get("port")?;
let port = Port::new(port_value).map_err(|e| ConfigError::ConversionError(e))?;

// 或使用 get_or 并进行验证
let port = Port::new(config.get_or("port", 8080u16))
    .map_err(|e| ConfigError::ConversionError(e))?;
```

对于更高级的类型转换，您可以实现 `prism3_value` 中的 trait（`MultiValuesFirstGetter`、`MultiValuesSetter` 等）。有关为自定义类型实现这些 trait 的详细信息，请参阅 `prism3_value` 文档。

## API 设计哲学

### 为什么选择纯泛型 API？

我们采用纯泛型方案（只提供 `get<T>()`, `set<T>()`, `get_or<T>()` 核心方法），而不是为每个类型提供专门的方法（如 `get_i32()`, `get_string()` 等），原因如下：

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
    TypeMismatch { expected: DataType, actual: DataType }, // 类型不匹配
    ConversionError(String),            // 类型转换失败
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

详细的 API 文档请访问 [docs.rs/prism3-config](https://docs.rs/prism3-config)。

内部设计文档请参阅 [src/README.md](src/README.md)。

## 依赖项

- `prism3-core` - 核心工具和数据类型定义
- `prism3-value` - 值处理框架
- `serde` - 序列化框架
- `chrono` - 日期和时间处理
- `regex` - 正则表达式支持

## 发展路线图

- [ ] 支持配置文件加载（XML, TOML, YAML, JSON）
- [ ] 支持配置合并策略
- [ ] 支持配置监听和热重载
- [ ] 支持配置验证框架
- [ ] 支持配置加密
- [ ] 提供线程安全的包装类型 `SyncConfig`

## 贡献

欢迎贡献！请随时提交 Pull Request。

## 许可证

Copyright (c) 2025 3-Prism Co. Ltd. All rights reserved.

根据 Apache 许可证 2.0 版（"许可证"）授权；
除非遵守许可证，否则您不得使用此文件。
您可以在以下位置获取许可证副本：

    http://www.apache.org/licenses/LICENSE-2.0

除非适用法律要求或书面同意，否则根据许可证分发的软件
按"原样"分发，不附带任何明示或暗示的担保或条件。
有关许可证下的特定语言管理权限和限制，请参阅许可证。

完整的许可证文本请参阅 [LICENSE](LICENSE)。

## 贡献

欢迎贡献！请随时提交 Pull Request。

## 作者

**胡海星** - *棱芯科技有限公司*

---

有关 Prism3 生态系统的更多信息，请访问我们的 [GitHub 主页](https://github.com/3-prism)。
