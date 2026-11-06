# qubit-config 用户手册

[English](user_guide.md) | 简体中文

本手册针对 `qubit-config` `0.15`。读者是需要从多个来源加载配置、以类型化方式读取配置，并在输入无效时获得可诊断错误的 Rust 应用开发者。本手册不要求业务代码绑定到某一种文件格式。

## 从 qubit-config 0.14 迁移

单键参数改为使用实现 `AsRef<str>` 的值（通常是 `&str` 或 `String`）。已移除的 `ConfigName` 类型应替换为字符串键本身；需要搜索多个候选键的 API 继续使用 `ConfigNames`。将 `IntoConfigDefault` 替换为 `qubit_value::IntoValueDefault`。本 crate 不再导出旧 trait；如果应用代码显式引用新 trait，应直接依赖与本 crate 的 `0.13` 要求对齐的 `qubit-value`。

跨 crate 边界传递配置类型前，先检查依赖图：

```sh
cargo tree -d
cargo tree -i qubit-config@0.14.0
cargo tree -i qubit-config@0.15.0
```

2026-11-03 核对的本地清单中，`rs-ioc` 将本地 `0.15` crate 声明为可选的普通依赖；`rs-http`、`rs-web`、`rs-websocket`、`rs-mime` 的普通依赖也指向本地 `0.15` crate，其中部分为可选依赖。`rs-magika` 则将相同路径和版本声明为开发依赖。这是本地清单快照，不代表下游已发布版本的依赖状态。

本 crate 使用 `qubit-value` `0.13`。

`^0.14` 不会自动接受 `0.15`，两个版本公开的 `Config` 或 `ConfigReader` 类型也彼此不同，不能跨版本传递。依赖图命令中的 selector 应替换为 lockfile 中实际出现的完整版本。优先使用已对齐到 `0.15` 的下游 crate。如果尚无可用版本，请保留兼容的依赖组合，或另开经授权的任务迁移下游；不要使用违反依赖约束的 `patch` 强行统一版本。

## 手册目标与读者

当应用具有如下配置生命周期时，可以阅读本手册：

1. 加载提交到仓库或生成的基础配置；
2. 从配置文件或进程环境变量叠加部署环境值；
3. 在应用组件中读取最终配置；
4. 对缺失、格式错误、冲突或超出大小限制的输入报告有用的路径信息。

本手册覆盖 `qubit_config` 的公共 API，不描述私有模块，也不对公共 API 和测试无法证明的行为作出承诺。

## 概念模型

`qubit-config` 将存储、读取、转换和加载分开：

| 概念 | 职责 |
| --- | --- |
| `Config` | 持有配置项，提供修改、配置源合并、序列化和根读取作用域。 |
| `Property` | 保存一个规范配置键、标量或集合值容器、可选描述和 final 标记。 |
| `ValueContainer` | 保留配置源提供的是标量还是显式集合；集合元素会分别转换。 |
| `ConfigReader` | 由 `Config` 和 `ConfigSection` 实现的封闭（sealed）只读接口，提供类型化、可选、默认值、多候选键、列表和严格读取。 |
| `ConfigSection` | 在点分路径下提供严格相对配置键的借用读取视图。 |
| `ReadPolicy` | 控制字符串、布尔值、集合、数值、Duration 和插值读取行为。 |
| `ConfigSource` | 产生独立的 `Config` 配置层，可检查，也可合并到目标配置。 |
| `CompositeConfigSource` | 按添加顺序加载多个配置源；同一个配置键的后加载配置层覆盖前一个，除非配置项是 final。 |

需要区分两种转换方式：

- `ConfigReader` 的 `get::<T>` 等方法通过 `FromConfig` 把一个存储的 property 转换为目标类型。
- `ConfigSerdeExt` 和 `Config::deserialize` 把一个精确 property 或 subtree 投影为由 Serde 管理的类型。

根 `Config` 和 section 共享相同的读取方法，但配置键作用域不同。在 `server` 创建的 section 中，`host` 会解析为 `server.host`；它不会把恰好名为 `server` 的标量配置项暴露为子项。

### 公共 API 分层

当前建议作为稳定核心依赖的 API 是 `Config`、`ConfigReader`、
`ConfigSection`、`ReadPolicy` 和 `ConfigSerdeExt`。这些类型覆盖配置持有、
类型化读取、作用域视图、转换策略和结构化反序列化。

`PropertiesConfigSource`、`EnvConfigSource`、`TomlConfigSource` 和
`YamlConfigSource` 等配置源适配器属于独立加载层，应按应用实际需要的输入格式选择
和启用。持久化与 wire 解码，以及低层 `Property` 操作，属于需要具体能力时再使用的
外围 API。当前稳定核心不包含热加载框架、异步配置源、对象安全 reader 或模式定义
DSL。

## 实战场景：基础配置叠加部署覆盖

假设服务有如下本地基础配置：

```properties
server.host=localhost
server.port=8080
server.timeout=30
```

部署环境可以设置 `APP_SERVER__HOST` 和 `APP_SERVER__PORT`。成功标准是：

- 没有覆盖值时，本地配置仍然可用；
- 环境变量只覆盖同名配置键；
- 应用通过相对 section 读取 `server`；
- 格式错误的值返回错误，而不是静默选择默认值。

下面的示例先使用内存中的 `.properties` 配置层以保证确定性，再加入进程环境变量配置层。`EnvConfigSource::from_prefix` 会选择 `APP_`，去除此前缀，将剩余部分转成小写，并把双下划线转换为点号，单个下划线保留。

```rust
use qubit_config::{Config, ConfigReader};
use qubit_config::source::{
    CompositeConfigSource, EnvConfigSource, PropertiesConfigSource,
};

fn load_server() -> Result<(String, u16, u64), Box<dyn std::error::Error>> {
    let mut sources = CompositeConfigSource::new();
    sources.add(PropertiesConfigSource::from_content(
        "server.host=localhost\nserver.port=8080\nserver.timeout=30\n",
    ));
    sources.add(EnvConfigSource::from_prefix("APP_"));

    let mut config = Config::new();
    config.merge_properties_from_source(&sources)?;
    let server = config.section("server")?;

    Ok((
        server.get("host")?,
        server.get("port")?,
        server.get_or("timeout", 30)?,
    ))
}
```

运行应用时设置 `APP_SERVER__PORT=9090`，即可观察到环境变量覆盖。只存在于 properties 配置层的配置键仍然可见。如果设置 `APP_SERVER__PORT=not-a-number`，`server.get::<u16>("port")` 会返回转换错误；`get_or` 不会掩盖已存在但无效的值。

## 安装与最小配置

crate 要求 Rust `1.94` 或更高版本，并使用 edition `2024`。

```toml
[dependencies]
qubit-config = "0.15"
```

默认 feature 集为空。按需显式添加可选能力：

```toml
# TOML 与 .env 配置源
qubit-config = { version = "0.15", features = ["toml", "env-file"] }
```

```toml
# Chrono 与 URL 值
qubit-config = { version = "0.15", features = ["chrono", "url"] }
```

```toml
# 所有可选值类型与格式配置源
qubit-config = { version = "0.15", features = ["full"] }
```

原子可选 feature 为 `bigdecimal`、`chrono`、`num-bigint`、`url`、`env-file`、`toml` 和 `yaml`。`rich-types` 组合前四个富类型 feature；`formats` 组合三个格式 feature；`full` 同时启用两组 feature。

结构化反序列化和 JSON 持久化示例还需要直接添加 Serde 依赖：

```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

最小的内存配置如下：

<!-- example: config_guide_quickstart -->
```rust
//! Shows the smallest Config write and typed-read workflow.

use qubit_config::Config;

/// Writes a server port and reads it back as an unsigned integer.
fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("server.port", 8080)?;
    let port: u16 = config.get("server.port")?;
    assert_eq!(port, 8080);
    Ok(())
}
```

## 核心工作流

### 写入和读取值

配置键必须是规范的非空点分名称。`server` 和 `server.port` 合法；`.server`、`server.` 和 `server..port` 会被拒绝，不会自动去除空白或规范化。

```rust
use qubit_config::{Config, ConfigError};

fn main() -> Result<(), ConfigError> {
    let mut config = Config::new();
    config.set("worker.threads", 4u16)?;
    config.set("worker.labels", ["api", "critical"])?;

    let threads: u16 = config.get("worker.threads")?;
    let labels: Vec<String> = config.get_list("worker.labels")?;
    let missing: u16 = config.get_or("worker.timeout", 30)?;

    assert_eq!(threads, 4);
    assert_eq!(labels, ["api", "critical"]);
    assert_eq!(missing, 30);

    config.set("worker.invalid", "abc")?;
    let error = config.get_or::<u16>("worker.invalid", 30).unwrap_err();
    assert_eq!(error.kind(), qubit_config::ConfigErrorKind::Conversion);
    assert!(matches!(error, ConfigError::ConversionError { .. }));
    Ok(())
}
```

缺失值的关键规则如下：

- 配置键缺失时，可以使用 `get_optional`、`get_or` 及其多候选键变体；
- unset 配置项或标量空白字符串在当前 `ReadPolicy` 下可能被视为实际缺失；
- 显式空集合仍然存在，因此 `get_optional_list` 返回 `Some(Vec::new())`；
- 已存在但无法转换的值会立即返回错误。

需要候选名称时，使用 `get_any`、`get_optional_any` 或 `get_any_or`。它们按照传入顺序检查配置键，并选择第一个不被视为缺失的值。

### 使用只读 reader 和 section

只消费配置的代码可以接收 `&impl ConfigReader`：

```rust
use qubit_config::{Config, ConfigReader, ConfigResult};

fn read_endpoint(reader: &impl ConfigReader) -> ConfigResult<(String, u16)> {
    let server = reader.section("server")?;
    Ok((server.get("host")?, server.get("port")?))
}

fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("server.host", "localhost")?;
    config.set("server.port", 8080i32)?;
    assert_eq!(read_endpoint(&config)?, ("localhost".to_owned(), 8080));
    Ok(())
}
```

`ConfigSection` 是严格相对的，可以继续嵌套：

组件需要限定允许的名称、同时保留明确开放的子 section 时，可以显式调用
`ensure_known_keys`。它只检查键名，不转换值，也不改变 reader 默认的宽松行为。
section 中的声明相对于当前 reader：

```rust
reader.ensure_known_keys(&["port", "max_connections"], &["proxy"])?;
```

根 reader 也可使用相同 API。未知路径以根配置的点分路径报告。

<!-- example: config_sections -->
```rust
//! Demonstrates navigating nested configuration sections with readers.

use qubit_config::Config;
use qubit_config::ConfigReader;

/// Reads a nested TLS setting through successive section views.
fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("server.tls.enabled", true)?;
    let server = config.section("server")?;
    let tls = server.section("tls")?;
    assert!(tls.get::<bool>("enabled")?);
    Ok(())
}
```

判断点分 section 是否存在时使用 `contains_section("server.tls")`。只有明确需要原始字符前缀匹配时才使用 `contains_key_prefix("server")`；它也可能匹配 `server2` 等同名前缀。路径敏感的 `section`、`contains`、`get_property`、`is_unset`、`remove` 和 `ConfigReader::resolve_key` 都返回 `ConfigResult`，因为非法路径是可观察错误。

### 加载和合并配置源

默认 `ConfigSource::load` 和每个库管理加载请求都会生成独立配置层。自定义配置源实现
`load_into(&mut SourceLoadContext)`，必须把 context 作为唯一输出入口。
默认的 `ConfigSource::load` 方法会通过标准预算和事务配置层执行器加载配置源。`Config::from_source`、
`merge_properties_from_source` 和组合来源等库管理入口始终使用该 executor 并调用 `load_into`，即使
来源覆盖了 `load` 也一样。直接调用覆盖的 `load` 由实现者负责；需要经库管理入口执行的自定义逻辑应迁入
`load_into`。
每次赋值应调用 `context.set(...)`；外部输入和解析工作应在执行前通过对应的记账方法
报告。框架无法从最终配置层推断遗漏的外部 I/O 或解析工作。内置配置源包括：

- 始终可用的 `PropertiesConfigSource`，支持 `.properties` 文件或内存内容；
- 始终可用的 `EnvConfigSource`，读取进程环境变量；
- 由 `toml` feature 启用的 `TomlConfigSource`；
- 由 `yaml` feature 启用的 `YamlConfigSource`；
- 由 `env-file` feature 启用的 `EnvFileConfigSource`；
- 按顺序合并其他配置源的 `CompositeConfigSource`。

在 `.env` 输入中，一个或多个空白字符可将赋值与行尾 `#` 注释分隔开。
注释中的引号、转义符或占位符不会影响下一条赋值；`$NAME` 和 `${NAME}` 仍保留为字面值。

需要让诊断显示稳定来源名称时，可在 properties、TOML、YAML、`.env` 或组合配置源的
builder 上调用 `label`。标签会原样成为来源标识，只用于诊断，不会写入配置或 Wire V1。
标签不要求唯一；如果需要分辨具体预算层，请为嵌套来源分别命名。不要把凭据或其他秘密放进
标签。省略标签会保留原有标识，`.label("")` 会恢复默认值。无论 `label` 放在 `file`／`content`
之前还是之后，最后一次调用都会生效。

```rust
use qubit_config::source::{CompositeConfigSource, PropertiesConfigSource};

let defaults = PropertiesConfigSource::builder()
    .label("defaults")
    .content("service.port=8080")
    .build();
let sources = CompositeConfigSource::builder()
    .label("service-config")
    .add_source(defaults)
    .build();
```

嵌套预算拒绝扣费时，`source_id()` 指向正在处理数据的叶子来源，`source_budget_id()` 指向
拒绝该扣费的局部来源或组合来源。例如 `leaf-b` 的赋值被 `outer` 组合来源拒绝时，来源标识是
`leaf-b`，预算作用域是 `outer`。标签不会改变来源顺序、final 校验或预算记账。
特别是来源数拒绝时，`source_id()` 指向尝试准入子来源的组合来源，
`source_budget_id()` 指向拒绝该次来源数扣费的作用域。

properties 解析器遵循 Java properties 的转义规则。它会解码
`\t`、`\n`、`\r`、`\f`、被转义的分隔符和空格、合法的 `\uXXXX`
UTF-16 code unit 以及合法的 surrogate pair。像 `\u12G4` 这样错误或
不完整的 Unicode 转义会原样保留；未知的非 Unicode 转义会按
Java properties 行为去掉前导反斜杠。

properties 物理行支持 LF、CRLF 和 CR 作为行结束符。格式空白仅包括 ASCII 空格、TAB 和
换页符；键和值中的其他 Unicode 空白会原样保留。若值中需要真正的回车符，可写成 `\r`。

`EnvFileConfigSource` 会把 `$NAME` 和 `${NAME}` 占位符作为字面值保留，注释后的值也不会
触发进程环境变量展开。加载 `.env` 文件不会隐式读取进程环境变量；应通过显式的
`*_interpolated` 读取策略在后续读取时解析。YAML anchor 和 alias 会在 YAML backend
构建 AST 前由 token scanner 拒绝。引号字符串、注释和块标量文本仍是普通标量内容；错误的
YAML scanner 输入也会直接失败，不会绕过 guard。所有内置配置源都通过
`SourceLoadContext` 加载；组合配置源会同时检查子配置源的局部策略与共享聚合策略，
全部通过后才扣减。`set` 和 `set_null` 已分别记入一个 node 和一个 property assignment；
只有额外解析工作才调用 `consume_nodes`。每次 charge 会先同时检查局部和聚合预算，但失败前已
接受的 charge 不会回滚。失败的配置层会被丢弃，调用方应传播错误。`ConfigSource` API 文档
提供了完整的内存配置源示例。TOML 和 YAML 会在调用 AST backend 前累计 parser token 并检查
parser nesting；原有 node、配置项数量和语义输出深度仍在展平阶段检查。解析准入预算不保证严格
RSS 或 CPU 上限：scanner 可能在 charge 前为一个大 token 分配内存，通过准入的输入仍会构造 AST。

自定义配置源可以在写入属性后通过 `context.set_final("service.mode", true)` 将其标记为 final。
此操作只修改属性元数据，不会再次扣减配置源预算。重复设置当前状态有效；final 属性不能解锁。
属性不存在或键无效时会返回错误。如果某个配置源拥有的值不应被更高优先级的后续来源覆盖，
可以这样使用：

```rust
use qubit_config::source::{ConfigSource, SourceLimits, SourceLoadContext};
use qubit_config::{ConfigError, ConfigResult};

struct FinalSource;
impl ConfigSource for FinalSource {
    fn source_id(&self) -> String { "managed-defaults".to_owned() }
    fn limits(&self) -> SourceLimits { SourceLimits::default() }
    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.set("service.mode", "managed")?;
        context.set_final("service.mode", true)
    }
}

let source = qubit_config::source::CompositeConfigSource::builder()
    .add_source(FinalSource)
    .add_source(qubit_config::source::PropertiesConfigSource::from_content(
        "service.mode=override\n",
    ))
    .build();
assert!(matches!(source.load(), Err(ConfigError::SourcePropertyIsFinal { .. })));
```

仅含注释或其他无值 token 的单个 YAML 文档会加载为空配置层；显式根 `null` 和 `~` 仍不是有效配置根。
TOML 和 YAML 也会在递归处理值之前拒绝空的或无效的点分 mapping 成员键；合法的空 table
和 mapping 可以成功解析，但不会生成 property，也不会保留 section 的存在性。例如 YAML
`server: {}` 和 TOML `[server]` 都会加载为空配置：`contains_section("server")` 为 false，
`section_if_present("server")` 返回 `None`；调用 `section("server")` 仍可创建空的只读视图。
空序列则不同，它仍是显式存在的 property。合并空 mapping 或 table 不会清除已有后代配置；
要求 `server` 子树的调用方仍会把仅含空对象的来源视为缺失。需要清空集合时使用空序列。
环境变量名称规范化后发生冲突时，只要输入有效且符合配置源预算，诊断会稳定选择规范化键最小的
冲突组及其中原始名称最小的两项。


下面的规范示例加载基础配置、应用明确的部署覆盖，并检查非法覆盖。它在子进程中使用
隔离前缀进行自验证；生产应用可以按自身环境约定选择前缀：

<!-- example: config_guide_sources -->
```rust
//! Demonstrates layered configuration sources with isolated environment
//! overrides.

use std::process::Command;

use qubit_config::Config;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigReader;
use qubit_config::ConfigResult;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::EnvConfigSource;
use qubit_config::source::PropertiesConfigSource;

/// Loads a baseline and an explicit deployment environment layer.
fn load_server() -> ConfigResult<(String, u16, u64)> {
    let sources = CompositeConfigSource::builder()
        .add_source(PropertiesConfigSource::from_content(
            "server.host=localhost\nserver.port=8080\n",
        ))
        .add_source(EnvConfigSource::from_prefix("QUBIT_CONFIG_GUIDE_APP_"))
        .build();
    let config = Config::from_source(&sources)?;
    let server = config.section("server")?;
    Ok((server.get("host")?, server.get("port")?, server.get_or("timeout", 30)?))
}

/// Checks one deployment state inside its own environment.
fn check_child(mode: &str) -> Result<(), Box<dyn std::error::Error>> {
    match mode {
        "baseline" => assert_eq!(load_server()?, ("localhost".to_owned(), 8080, 30)),
        "override" => assert_eq!(load_server()?, ("deployed.test".to_owned(), 9090, 30)),
        "invalid" => {
            let error = load_server().expect_err("invalid port must fail");
            assert_eq!(error.kind(), ConfigErrorKind::Conversion);
            assert_eq!(error.path(), Some("server.port"));
        }
        _ => return Err("unexpected example child mode".into()),
    }
    Ok(())
}

/// Runs baseline, deployment override, and invalid environment scenarios.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(mode) = std::env::args().nth(1) {
        return check_child(&mode);
    }
    for mode in ["baseline", "override", "invalid"] {
        let mut command = Command::new(std::env::current_exe()?);
        command.env_clear().arg(mode);
        if mode == "override" {
            command.env("QUBIT_CONFIG_GUIDE_APP_SERVER__HOST", "deployed.test");
            command.env("QUBIT_CONFIG_GUIDE_APP_SERVER__PORT", "9090");
        } else if mode == "invalid" {
            command.env("QUBIT_CONFIG_GUIDE_APP_SERVER__PORT", "not-a-number");
        }
        assert!(command.status()?.success(), "example child failed: {mode}");
    }
    Ok(())
}
```

如果不需要在加载前定制目标配置，可以使用便捷构造函数：

```rust,no_run
use qubit_config::Config;

fn main() -> Result<(), qubit_config::ConfigError> {
    let config = Config::from_properties_file("config.properties")?;
    assert!(config.contains("port")?);
    Ok(())
}
```

如果目标已有值或读取策略，则使用 `merge_properties_from_source`：

```rust
use qubit_config::source::{
    CompositeConfigSource, PropertiesConfigSource,
};
use qubit_config::Config;

fn main() -> Result<(), qubit_config::ConfigError> {
    let mut source = CompositeConfigSource::new();
    source.add(PropertiesConfigSource::from_content("port=8080\n"));
    source.add(PropertiesConfigSource::from_content("port=9090\n"));

    let mut config = Config::new();
    config.merge_properties_from_source(&source)?;
    assert_eq!(config.get::<i64>("port")?, 9090);
    Ok(())
}
```

配置源加载和合并在公共边界具有事务语义：配置源先加载到独立配置层，完整校验输入配置层后才合并；失败时目标配置保持不变。incoming 配置源合并时触发 final 冲突，返回 `SourcePropertyIsFinal`，其中包含合并边界的来源 ID 和键；稳定 kind 仍为 `PropertyIsFinal`。嵌套组合会保留内层合并边界的来源 ID。多个叶子被作为一个聚合层加载时，诊断定位到该聚合层，不伪称某个叶子来源。普通内存操作及来源内部同层直接赋值仍返回 `PropertyIsFinal`。

### 反序列化结构化值

`Config::deserialize` 会把精确 property 或点分 subtree 映射为由 Serde 管理的类型。空 prefix 表示根 map。

<!-- example: config_structured_guide -->
```rust
//! Demonstrates structured object, collection, and byte deserialization.

use qubit_config::Config;
use serde::Deserialize;
use serde::de;
use serde::de::Deserializer;
use serde::de::Visitor;

/// Server settings selected from a structured configuration section.
#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    /// Server host name.
    host: String,
    /// TCP service port.
    port: u16,
}

/// Byte sequence deserialized from a string value using serde bytes.
#[derive(Debug, PartialEq)]
struct Bytes(Vec<u8>);

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        /// Accepts byte buffers produced by the configured deserializer.
        struct BytesVisitor;

        impl Visitor<'_> for BytesVisitor {
            type Value = Bytes;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("bytes")
            }

            fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Bytes(value))
            }
        }

        deserializer.deserialize_bytes(BytesVisitor)
    }
}

/// Reads a server object, numeric vector, and text as bytes.
fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("server.host", "localhost")?;
    config.set("server.port", 8080)?;
    config.set("numbers", vec![65_u8, 66_u8])?;
    config.set("text", "AB")?;

    let server: Server = config.deserialize("server")?;
    assert_eq!(server.host, "localhost");
    assert_eq!(server.port, 8080);

    let numbers: Vec<u8> = config.deserialize("numbers")?;
    assert_eq!(numbers, vec![65_u8, 66_u8]);

    let bytes: Bytes = config.deserialize("text")?;
    assert_eq!(bytes.0, b"AB");
    Ok(())
}
```

`Vec<u8>` 请求 Serde sequence，因此数值序列 `[65, 66]` 会映射为 `[65, 66]`。
请求 `deserialize_bytes` 或 `deserialize_byte_buf` 的目标则遵循配置到 `String` 的现有转换，
再取得该字符串的 UTF-8 字节。这不会隐式执行 Base64 或十六进制解码：文本 `"4142"`
得到的是这四个字符的 UTF-8 字节，而不是 `[0x41, 0x42]`。`serde_bytes::ByteBuf` 等字节包装
类型走 bytes 接口，因此与 `Vec<u8>` 具有不同的输入契约。针对 `serde_bytes` 0.11.19 的
兼容性核查确认，`ByteBuf` 可将已测试的字符串读取为 UTF-8 字节，但不接受已存储的数值序列
或精确 JSON 数组；需要数值序列输入时应使用普通 `Vec<u8>`。

通过泛型 `ConfigReader` 或 section 调用 `deserialize` 或 `deserialize_with` 时，导入
`ConfigSerdeExt`。`deserialize(prefix)` 等价于
`deserialize_with(prefix, ConfigDeserializeOptions::default())`：不插值；Serde 的
源反序列化器收到 ignored-value 请求时可观察到的字段会以排序后的 root-relative 路径返回为
`ConfigError::UnknownProperties`。这不等于完整 schema 校验：flatten struct、untagged enum
和自定义反序列化器可能缓冲 key 而不发出此类请求；flatten map 也可能消费任意 key。
闭合的普通 struct 可在目标上添加 `#[serde(deny_unknown_fields)]`；untagged enum 的每个
struct payload 也应添加。Serde 随后返回经过脱敏的反序列化错误，不保证是
`UnknownProperties`。Serde 禁止该属性与 `flatten` 组合；要求闭合 schema 时，应改用闭合的
嵌套 struct，或在反序列化前按应用 schema 校验 key。允许额外字段时设置
`unknown_fields: UnknownFieldPolicy::Ignore`；展开占位符时设置 `interpolate: true`。
struct 字段、`serde(rename)`、`serde(alias)`、`serde(default)`、嵌套 struct、map 和
`serde(flatten)` 共同声明可接受的形状。仅由 Serde 报告的结构不匹配会变成已脱敏的
`DeserializeError`。
标准缺字段错误会保留经 JSON 转义的 schema 字段名；类型错误使用固定摘要。输入值、Serde 的
`Expected` 和 `Unexpected` 描述以及自定义反序列化器消息都不会保留。库内部固定摘要可以报告
内部结构名称、已知完整长度或多余项信息。这些面向人的摘要可能变化；程序判断应使用
`ConfigError::kind()` 和 `ConfigError::path()`，不要依赖 `Display` 文本。

结构化读取诊断对普通成员名继续使用点分路径。含 `.`, `[`, `]`, `"`, `\\` 或控制字符的
成员名改用 JSON 字符串转义的方括号段：字面成员 `a.b` 显示为 `payload["a.b"]`，嵌套
成员仍显示为 `payload.a.b`；数组位置仍显示为 `[index]`。选中根路径中的特殊段也使用
相同编码。这些字符串仅用于诊断，不可传给 `Config::get` 或 section 查询。普通 reader 路径和
Wire JSON Pointer 保持各自格式。直接比较特殊键诊断字符串的调用方应更新比较值；配置键和查询
行为不变。

map 键反序列化失败时，诊断也会定位到具体成员。例如，根对象中键 `80` 的失败路径由空字符串变为
`80`，`tree` 下成员 `key` 的失败路径由 `tree` 变为 `tree.key`。直接比较这些诊断路径的调用方需要
更新断言；数值 map 键仍不会自动转换。

插值查找作用域取决于选中的存储形态。`prefix` 选中点分子配置项时，先在选中的子树中查找；
选中 JSON 对象等精确结构化属性时，则从调用 reader 的作用域开始查找。对象成员名只是数据，
不会成为 Config key。原有根配置和允许的环境变量回退规则仍然适用。

对象成员名会以字符串形式交给 Serde。若应用需要数字 key，应先读取字符串 key，
再显式解析。规范示例会拒绝 `080` 和 `80` 这类归一化后重复的 key：

<!-- example: config_numeric_map_keys -->
```rust
//! Shows numeric map-key conversion and rejection of normalized duplicates.

use std::collections::BTreeMap;

use qubit_config::Config;

/// Converts string keys while rejecting normalized numeric duplicates.
fn numeric_ports(named: BTreeMap<String, u16>) -> Result<BTreeMap<u16, u16>, Box<dyn std::error::Error>> {
    let mut ports = BTreeMap::new();
    for (name, value) in named {
        let number = name.parse::<u16>()?;
        if ports.insert(number, value).is_some() {
            return Err("duplicate numeric port key".into());
        }
    }
    Ok(ports)
}

/// Reads numeric port keys and verifies that 80 and 080 collide.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.set("ports.80", 8080_u16)?;
    config.set("ports.443", 8443_u16)?;
    let named: BTreeMap<String, u16> = config.deserialize("ports")?;
    let ports = numeric_ports(named)?;
    assert_eq!(ports.get(&80), Some(&8080));
    assert_eq!(ports.get(&443), Some(&8443));
    config.set("ports.080", 8081_u16)?;
    let duplicate: BTreeMap<String, u16> = config.deserialize("ports")?;
    assert!(numeric_ports(duplicate).is_err());
    Ok(())
}
```

即使目标忽略部分字段，也会对选中的完整输入执行准入。插值前后的输入分别检查，然后所有叶子
转换共享另一个独立的 `ConversionSession`。这些资源域都取 reader 的转换限制，被忽略字段
不能绕过输入限额。只有真正的字符串叶子参与插值，富类型格式化的文本不参与。
内部未变化的值保持借用，返回结果仍拥有所有权。

明确的数值字段从原始运行时类型转换。`serde_json::Value` 等使用 `deserialize_any` 的目标
遵循自然 JSON 类别：宽整数是十进制字符串，富类型使用自然表示。并非所有富类型都接受该
Serde 形状，例如 `std::time::Duration` 需要 `secs` 和 `nanos` 字段，不接受存储的标量 Duration。
JSON 整数字面量在有符号或无符号 128 位范围内精确保留；更大的整数字面量和非有限浮点值会
返回转换错误，有限浮点值沿用既有二进制浮点语义。这条转换路径与通用 `Config` wire
反序列化器不同；依赖图统一启用 `serde_json/arbitrary_precision` 时后者的行为见下文持久化说明。
不同的对象叶子可与点分属性合并；重复叶子、精确属性与其子项冲突会返回 `KeyConflict`。
子树中被策略判定缺失的标量字符串会被省略，集合某项缺失仍然报错。

### 持久化和解码配置

`Config` 通过稳定的、带版本的 V1 JSON wire 格式实现 Serde 序列化；解码时也继续接受旧的无版本载荷。面对完整的不可信输入时，可以使用带默认结构限制的解码器：

```rust
use qubit_config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.set("server.port", 8080)?;
    let bytes = config.encode_json_vec()?;
    let restored = Config::decode_json_slice(&bytes)?;
    assert_eq!(restored.get::<i64>("server.port")?, 8080);
    Ok(())
}
```

当默认限制配置不适用时，`Config::encode_json_vec_with_limits` 和
`Config::decode_json_slice_with_limits` 都接收 `ConfigWireLimits`。该
限制配置组合 rs-budget 的定向 `JsonDecodeLimits`、`JsonEncodeLimits` 与配置专属的
配置项、配置项键限制。输入通过一个 `JsonDecodeSession` 准入，输出结构和字节
通过一个 `JsonEncodeSession` 记账；配置专属限制仍由本 crate 处理。
从 `ConfigWireLimits::builder_from(&limits)` 开始调用 `max_input_bytes` 时，只修改输入字节限制，
并保留其他解码维度。需要整体替换解码 profile 时使用 `json_decode`。
普通 Serde 序列化的行为保持不变。该 JSON 预算与读取文本配置源时
使用的 `SourceLimits` 相互独立。普通 `Deserialize` 会用
`ConfigWireLimits::default()` 创建有预算约束的已解码值 visitor，从而限制已解码结构、
载荷、配置项数量和配置项键。通用 Serde 反序列化器不提供原始字节流或 JSON 词法
token，因此普通 `Deserialize` 无法执行原始输入限制。即使依赖图统一启用
`serde_json/arbitrary_precision`，普通 Serde 也会按 schema 角色读取原生 Float32/Float64 wire 字段；
任意 JSON 领域数字仍遵循通用事件语义，需要保留 JSON 数字语义时使用专用 JSON helper。不可信 JSON 应使用
`Config::decode_json_slice` 或其自定义限制版本。V1 拒绝显式 `read_options` 字段，
包括 `null`；旧版无版本输入仍接受并忽略该字段。手工构造 V1 封装时应省略该字段。

普通 Serde 解码中，类型化浮点字段的早期错误会替换为安全的通用消息；配置数值
检查产生的错误仍保留安全的预算或表示摘要。通过 Serde 重建错误可能改变错误类别
或源码坐标。需要稳定的语法、预算、表示分类及 JSON Pointer 诊断时，应使用有界 slice API。

格式错误的 wire 表示会返回
`ConfigWireDecodeError::Representation { path, reason }`。路径是相对于 wire 文档根的
JSON Pointer，例如 `/properties/server.port/value`；`server.port` 中的点属于同一个
property 名称。JSON Pointer 会把 `~` 编码为 `~0`，把 `/` 编码为 `~1`。这与类型化读取
返回的 `server.port` 等点分配置路径不同。有界预检会识别缺少字段、封装字段类型错误和
未知字段；其他深层解码失败统一使用安全的通用原因 `InvalidValue`。解码器不会包含无效
输入值，也不会伪造源码行列坐标。此前归为 `Json(Data)` 的业务表示失败现在归为
`Representation`；匹配旧分类的调用方应处理此变体。语法和预算错误仍保留原有分类。

`usize` 支持标量类型化读取，也可用于可选值和默认值读取：

```rust
let maximum = reader.get_optional::<usize>("max_connections")?;
let fallback = reader.get_or("max_connections", 1024_usize)?;
```

转换沿用现有 `u128` 策略，再检查当前目标平台的范围。零到 `usize::MAX` 均可接受；
更大的值返回带已解析 key 的转换错误，集合首项还会保留来源索引。`usize` 不是存储或
wire 整数类型，本次也不增加 `Vec<usize>` 转换。

下游可用 `ensure_known_keys` 替换重复的未知键检查，用 `get_optional::<usize>` 替换
重复的平台范围检查。例如：

```rust
reader.ensure_known_keys(&["port", "max_connections"], &["proxy"])?;
let maximum = reader.get_optional::<usize>("max_connections")?;
let fallback = reader.get_or("max_connections", 1024_usize)?;
```

这些示例使用 `qubit-config` 0.15 中提供的 API。仍使用 0.14 的调用方在跨 crate
边界传递配置类型前，应先对齐依赖版本。

V1 和旧版持久化都不保存运行时默认 `ReadPolicy`。带限制的 JSON 解码和普通 Serde
反序列化都会恢复为 `ReadPolicy::default()`。`Config` 相等性只比较持久化数据，因此
数据往返相等不表示自定义读取策略也被保留。需要时显式恢复策略，或用 `read_with` 创建临时视图：

<!-- example: config_policy_restore -->
```rust
//! Demonstrates that read policy is restored separately from serialized config
//! data.

use qubit_config::Config;
use qubit_config::ConfigReader;
use qubit_config::ReadPolicy;

/// Restores a config and reapplies its original read policy explicitly.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let policy = ReadPolicy::builder_from(&ReadPolicy::env_friendly())
        .max_interpolation_depth(7)
        .build();
    let mut original = Config::builder().default_read_policy(policy.clone()).build();
    original.set("enabled", "yes")?;
    assert!(original.get::<bool>("enabled")?);
    let mut restored = Config::decode_json_slice(&original.encode_json_vec()?)?;
    assert_eq!(restored, original);
    assert_eq!(restored.default_read_policy(), &ReadPolicy::default());
    assert!(restored.get::<bool>("enabled").is_err());
    assert!(restored.read_with(&policy).get::<bool>("enabled")?);
    restored.set_default_read_policy(policy.clone());
    assert_eq!(restored.default_read_policy(), &policy);
    assert!(restored.get::<bool>("enabled")?);
    Ok(())
}
```

### 选择读取策略

`Config` 为直接读取持有默认 `ReadPolicy`。`read_with` 创建临时借用视图，在不修改配置的情况下使用另一套 policy：

<!-- example: config_read_policy -->
```rust
//! Demonstrates per-read environment-friendly parsing without changing
//! defaults.

use qubit_config::Config;
use qubit_config::ConfigReader;
use qubit_config::options::ReadPolicy;

/// Reads environment-style values with a temporary policy.
fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("HTTP_ENABLED", "yes")?;
    config.set("HTTP_PORTS", "8080, 8081,,8082")?;

    let policy = ReadPolicy::env_friendly();
    let reader = config.read_with(&policy);
    let enabled: bool = reader.get("HTTP_ENABLED")?;
    let ports: Vec<u16> = reader.get("HTTP_PORTS")?;
    assert!(enabled);
    assert_eq!(ports, [8080, 8081, 8082]);
    assert_eq!(config.default_read_policy(), &ReadPolicy::default());
    Ok(())
}
```

`ReadPolicy` 组合控制字符串空白处理、布尔字面量、集合拆分、数值转换、Duration 转换和插值限制。应用可以使用 builder 方法选择更严格或不同的策略。传入完整的 `StringConversionPolicy` 或 `CollectionConversionPolicy` 时，该嵌套策略会原样替换；叶级 `blank_string_policy` 和 `empty_item_policy` setter 只修改各自对应的嵌套设置。底层转换选项类型由 `qubit-datatype` 提供；如果应用直接配置这些低层选项，也应直接依赖该 crate。

`ConversionOperationLimits::max_input_bytes` 限制进入一次转换操作的输入。使用
`BlankStringPolicy::TreatAsMissing` 时，普通标量空白值可能在调用 `FromConfig` 前就被预检查为
`None` 或默认值；这个预检查不计入转换输入字节预算。非空白值会进入转换并受该预算约束。
这不表示预检查的 CPU 时间有严格上限。结构化读取有独立的准入边界：完整选中的 JSON 值或子树会在
目标反序列化前检查，包括其中的空白字符串成员。这属于不同资源域；结构化输入字节预算拒绝某个
对象，不代表普通可选读取也会拒绝相同的空白标量。

存储形态也会影响结果。像 `[" "]` 这样的字符串集合会进入逐项转换，因此缺失策略可能返回带元素
索引的 missing/value 错误；显式空集合仍可成功。插值读取会预检查原始空白文本以及展开后变成空白的
文本，但插值展开次数等限制仍然生效。结构化读取会先对完整输入准入，无论输入来自精确 JSON 属性还是
点分子树。预算足够时，精确属性中的空白 `Option<String>` 字段报告缺失，而对应的点分子树可以得到
`None`。

### 显式执行插值

普通的 `get` 和 `deserialize` 会原样保留 `${host}` 这样的占位符。只有当插值是配置契约的一部分时，才使用对应的 interpolated 方法：

<!-- example: config_interpolation -->
```rust
//! Shows how interpolated reads expand references while raw reads preserve
//! them.

use qubit_config::Config;

/// Reads one URL both literally and after resolving its host reference.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.set("host", "localhost")?;
    config.set("url", "http://${host}")?;

    assert_eq!(config.get::<String>("url")?, "http://${host}");
    assert_eq!(config.get_interpolated::<String>("url")?, "http://localhost",);
    Ok(())
}
```

默认插值来源是 `ConfigOnly`。如果要回退查询进程环境变量，必须显式配置：

```rust
use qubit_config::{Config, ConfigReader};
use qubit_config::options::{InterpolationSources, ReadPolicy};

let policy = ReadPolicy::builder_from(&ReadPolicy::env_friendly())
    .interpolation_sources(InterpolationSources::ConfigThenEnv)
    .build();
let config = Config::builder().default_read_policy(policy).build();
let _ = config;
```

能够选择环境变量名称的配置应被视为受信任输入。默认插值限制为 64 层引用链深度、
每次读取 4,096 次占位符展开，以及 1 MiB UTF-8 输出。可以通过
`ReadPolicy::builder()` 分别调整；超限会返回结构化 `ConfigError` 类别。
插值使用显式堆帧栈遍历引用，不再为每层引用各占一个 Rust 调用栈帧。活动帧存储会随引用链增长；现有深度、展开次数和输出字节限制仍生效，但不承诺严格的进程内存或 CPU 上限。
占位符展开次数会在一次逻辑读取内共享累计，包括候选键搜索和结构化 visitor 最终忽略的字段。
深度限制按当前引用链计算，输出字节限制则应用于每个展开值。
引用值转成文本时使用当前 `ReadPolicy` 及其转换限制；每次引用转换使用独立的转换操作，
一次逻辑读取内的插值展开预算仍会累计共享。
插值字符串集合会按源顺序逐项展开并转换，所有元素共享同一个转换会话。遇到首个插值或
转换错误时立即返回，并保留原集合索引；处理过程中只持有一个已展开的元素。集合元素仍是
独立值，标量文本仍按配置的列表拆分策略处理。现有调用方式和有效输入无需迁移。

### Custom Target Types

为应用类型实现 `FromConfig` 时，可以委托给已有的内置转换。读取器会提供解析上下文，
因此委托转换会沿用读取策略，并在错误中保留解析后的配置键。插值仍需显式开启：普通读取
会保留 `${default_port}` 并因无法转换而报错；带作用域的插值读取则会相对于 `server` 解析它。

<!-- example: config_custom_target -->
```rust
//! Demonstrates custom value conversion with interpolation and error context.

use qubit_config::Config;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigReader;
use qubit_config::ConfigResult;
use qubit_config::Property;
use qubit_config::conversion::ConfigParseContext;
use qubit_config::conversion::FromConfig;

/// Wraps a service port parsed from a configuration property.
#[derive(Debug, PartialEq)]
struct ServicePort(u16);

impl FromConfig for ServicePort {
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        u16::from_config(property, ctx).map(Self)
    }
}

/// Exercises custom conversion for direct, interpolated, and invalid values.
fn main() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("server.port", "8080")?;
    assert_eq!(config.get::<ServicePort>("server.port")?, ServicePort(8080));

    config.set("server.default_port", "9090")?;
    config.set("server.port", "${default_port}")?;
    let server = config.section("server")?;
    assert_eq!(server.get_interpolated::<ServicePort>("port")?, ServicePort(9090));
    assert!(server.get::<ServicePort>("port").is_err());

    config.set("server.port", "invalid")?;
    let error = config.get::<ServicePort>("server.port").unwrap_err();
    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("server.port"));
    Ok(())
}
```

这段示例覆盖四项检查：普通类型化读取、显式的 section 插值、普通读取保留占位符而转换失败，
以及非法值错误保留根配置路径。`ServicePort` 仅是应用示例；端口 0 是否有效属于应用的业务校验。

### 规范化环境变量配置键

`EnvConfigSource::from_prefix("APP_")` 依次执行：

1. 选择以 `APP_` 开头的名称；
2. 去除前缀；
3. 将剩余名称转为小写；
4. 把 `__` 转换为 `.`，单个 `_` 保留。

例如，`APP_DATABASE__MAX_CONNECTIONS` 会变成 `database.max_connections`。如果只需要部分转换，可以使用 `EnvConfigOptions`。如果两个不同的环境变量名称经过规范化后变成同一个配置键，加载会返回 `KeyConflict`，不会静默选择其中一个；错误会按字典序报告冲突名称，因此诊断信息不依赖操作系统的环境变量遍历顺序。

Unicode 名称的前缀匹配区分大小写。在 Unix 上，非 Unicode 名称按原始字节与前缀比较；在 Windows 上，则按 UTF-16 代码单元比较。匹配前缀的非 Unicode 名称会返回脱敏的解析错误，无关名称仍会跳过。其他平台无法通过标准库检查名称的原生单元，因此带前缀加载时会跳过非 Unicode 名称。如果 Windows 部署中存在匹配前缀的无效名称，请修正该名称，或调整前缀以排除它。

TOML 和 YAML 配置源会把映射展开成点分配置项，接受标量、空序列和同类型的字符串、布尔值或
现有同一数值类别标量序列。TOML 字符串与日期／时间值可以混合成数组，并统一存为字符串集合；
此例外不会改变 YAML 的同类型序列规则，也不允许其他类别混合。YAML 序列不支持 null 元素；对象数组、嵌套数组和异构标量序列
也会携带配置源、路径和索引上下文被拒绝，不会通过隐式字符串化掩盖结构不匹配。

### 配置源限制

默认 `SourceLimits` 如下：

| 限制 | 默认值 |
| --- | ---: |
| 输入字节数 | 8 MiB（`8 * 1024 * 1024`） |
| 输出 assignment 数 | 65,536 |
| 解析结构节点数 | 262,144 |
| Parser token 数 | 1,048,576 |
| Parser nesting depth | 128 |
| 此作用域内累计准入的后代来源数（不含作用域根） | 256 |
| 嵌套深度 | 64 |

所有内置配置源的文件入口和内存入口使用同一套 session API。
`CompositeConfigSource::builder().limits(...).build()` 会在每个子配置源的局部策略之外增加共享聚合
策略；只有所有适用预算都接受一次累计扣减后才会统一记账。TOML 和 YAML 的节点、
配置项和语义输出深度检查仍发生在 AST 构建后的 flatten 阶段。`max_parser_tokens` 按 scanner/lexer
返回的 token 计数并在 source 与所有外层 composite 间累计；TOML 的空白、注释、换行和 EOF 也计入。
Parser nesting depth 对每个输入单独检查，根深度为 0；YAML 按 collection 层级计数，TOML 使用保守的
结构上界（每个点分 key 段最多计两层，array/inline 容器还会增加层级），因此不等同于语义 `max_depth`。
这些限制减少送入 backend 的工作，但不保证严格内存或 CPU 上限；单个大 token 和准入后的 AST 仍有成本。

`max_sources` 限制此作用域内累计准入的后代来源数，不计作用域根。进入一个嵌套组合来源会计 1，
该组合来源之后进入的每个来源也会在所有外层作用域各计 1。例如 `outer -> inner -> leaf-a, leaf-b`，
`outer` 的限额至少为 3，`inner` 至少为 2。每次加载都会创建新的预算。该计数限制准入的来源数，
不限制组合嵌套深度；提高该限额或使用 `SourceLimits::unbounded()` 不代表任意深链都能安全使用线程栈。
优先扁平化组合，并保留保守的有限限额。详见[组合来源资源探测记录](composite-resource-probe-20261106T065653653Z-81411945.md)，
其中说明了实测环境和适用边界。

在了解输入边界后，可用 builder 调整限制：

```rust
use qubit_config::source::SourceLimits;

let limits = SourceLimits::builder()
    .max_parser_tokens(65_536)
    .max_parser_nesting_depth(96)
    .build();
assert_eq!(limits.max_parser_tokens(), 65_536);
```

对应错误类别为 `SourceLimitKind::ParserTokenCount` 和
`SourceLimitKind::ParserNestingDepth`，错误仍是 `SourceLimitExceeded`。旧版序列化的
`SourceLimits` 缺少新字段时使用上表默认值；`SourceLimits::unbounded()` 也会关闭这两个新维度。
解析准入限额可能先于后续语法、node、assignment 或语义深度错误返回。自定义 parser 可在调用
backend 前通过 `SourceLoadContext::consume_parser_tokens` 和
`SourceLoadContext::check_parser_depth` 报告工作量。
只有应用完全控制输入边界时才应使用 `SourceLimits::unbounded()`。

## 错误与诊断

`ConfigResult<T>` 是 `Result<T, ConfigError>`。`ConfigError` 是 non-exhaustive，下游应使用稳定类别和上下文访问器：

```rust
use qubit_config::{Config, ConfigErrorKind, ConfigReader};

let config = Config::new();
let error = config.get::<u16>("server.port").unwrap_err();

assert_eq!(error.kind(), ConfigErrorKind::PropertyNotFound);
assert_eq!(error.path(), Some("server.port"));
```

常见类别包括 `InvalidKey`、`InvalidPath`、`PropertyNotFound`、`PropertyHasNoValue`、`Value`、`TypeMismatch`、`Conversion`、`Substitution`、`SubstitutionCycle`、`SourceLimitExceeded`、`KeyConflict`、`Merge`、`PropertyIsFinal`、`Io`、`Parse` 和 `Deserialize`。来源合并冲突使用 `SourcePropertyIsFinal` 变体，但 kind 仍是 `PropertyIsFinal`；普通内存操作和来源内部直接赋值保留 `PropertyIsFinal`。

对于 `get::<u16>` 这类内置标量类型的普通必需读取，以下情况会得到不同结果：

| 输入和读取条件 | 错误类别 | 判断方式及处理建议 |
| --- | --- | --- |
| 配置键不存在 | `PropertyNotFound` | 检查配置来源；再根据业务需要选择可选读取或默认读取。 |
| 空白字符串，且 `BlankStringPolicy::TreatAsMissing` 将其视为缺失 | `PropertyHasNoValue` | 读取策略的预检查将该值判为缺失；修正输入，或改用符合业务需求的策略。 |
| 通过 `set_null` 设置的 unset 标量 | `Value` | 检查 `value_missing()`，并可用 `is_unset()` 确认；并非所有 `Value` 错误都表示缺失。 |
| 非空字符串无法转换，例如 `not-a-port` | `Conversion` | 修正输入；不要用默认值掩盖转换失败。 |

`Value` 表示值层错误。`value_missing()` 返回 `Some` 时会提供缺失值上下文；unset 标量就是其中一种情况。其他 `Value` 错误不能自动视为缺失。下面的辅助函数由应用自行定义，用于识别缺失值，同时保留转换失败；它不是库提供的新 API：

```rust
use qubit_config::{Config, ConfigError, ConfigErrorKind};
use qubit_datatype::DataType;

fn is_missing_value(error: &ConfigError) -> bool {
    match error.kind() {
        ConfigErrorKind::PropertyNotFound | ConfigErrorKind::PropertyHasNoValue => true,
        ConfigErrorKind::Value => error.value_missing().is_some(),
        _ => false,
    }
}

fn main() -> Result<(), ConfigError> {
    let mut config = Config::new();
    config.set_null("server.port", DataType::String)?;
    let error = config.get::<u16>("server.port").unwrap_err();
    assert_eq!(error.kind(), ConfigErrorKind::Value);
    assert_eq!(error.path(), Some("server.port"));
    assert!(error.value_missing().unwrap().is_unset());
    assert!(is_missing_value(&error));

    config.set("server.port", "not-a-port")?;
    let error = config.get::<u16>("server.port").unwrap_err();
    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert!(!is_missing_value(&error));
    Ok(())
}
```

此矩阵仅适用于内置标量类型的普通必需读取。不要据此推断严格读取、集合读取、结构化读取或自定义 `FromConfig` 具有相同行为。`get_optional` 和 `get_or` 遵循不同的读取契约，应按应用需求选择。

对于配置源的 IO、解析、限制错误，以及 `SourceLoadContext::set` 或 `set_null`
产生的无效键错误，`source_id()` 会返回文件路径或稳定的配置源标识。此类赋值错误
使用 `SourceInvalidKey` 变体，同时保留 `InvalidKey` 类别和被拒绝的键路径。直接调用
`Config::set` 仍返回不带来源 ID 的 `InvalidKey`。其他错误是否带来源上下文由其变体决定。
匹配具体错误变体的代码需要处理 `SourceInvalidKey`；优先使用 `kind()`、`path()` 和
`source_id()` 可以减少对具体来源诊断变体的依赖。
配置源预算失败时，`source_budget_id()` 会指出拒绝扣减的局部或聚合预算，
`budget_error()` 则返回结构化的 rs-budget 错误。
保存的 overlay 也可按累计聚合预算准入；单个解析文本仍受单值限制。启用累计准入时，
预算超限错误可能优先于后续解析错误。

`get_any` 失败时可以使用 `candidate_paths()`，因为多候选键错误可能包含按查找顺序排列的多个路径。如果集合转换错误对应原始集合中的某个元素，可以使用 `source_index()` 获取其位置。

程序逻辑应使用 `kind()`、`path()` 和候选路径处理诊断。配置值的 `Debug` 输出会通过 `qubit-redact` 遮盖存储值，同时保留 property 元数据。

## 排障

### 默认值没有生效

检查配置键是否存在并且有值。默认值只适用于缺失或实际缺失的配置键；如果配置键存在但无法转换为目标类型，`get_or` 会立即返回 `ConversionError`。缺失值和转换失败的区别见[错误与诊断](#错误与诊断)。

### `${...}` 没有被替换

使用 `get_interpolated`、`get_interpolated_or`、`get_any_interpolated`，或为
`deserialize_with` 设置 `interpolate: true`。普通读取会有意保留占位符字面量。
如果占位符应来自进程环境变量，请确认当前 policy 使用了 `ConfigThenEnv`。

### section 无法读取配置键

检查 section 作用域和相对配置键。`config.section("server")?.get("port")` 解析为 `server.port`；section 不包含它自身的根标量。使用 `contains_section` 判断 section 成员，使用 `keys()` 查看当前可见的相对配置键。

### 文件 loader 不可用

检查 `Cargo.toml` 中的 feature：`toml` 启用 `TomlConfigSource`，`yaml` 启用 `YamlConfigSource`，`env-file` 启用 `EnvFileConfigSource`。`PropertiesConfigSource` 和 `EnvConfigSource` 不需要这些格式 feature。

### 配置源报告键冲突

检查环境变量规范化后的名称或结构化配置键展平结果。去前缀、转小写和下划线转换可能使不同名称合并为同一个配置键；TOML/YAML 文档也可能产生重复的展平配置键。请重命名输入，或减少规范化步骤。
当环境变量名称冲突时，只要输入有效且符合预算，诊断会稳定选择规范化键最小的冲突组及其中
原始名称最小的两项。
`KeyConflict` 的显示文本会转义控制字符；`path()` 等结构化字段仍保留原值，因此应使用
`kind()`、`path()` 等接口，而不要解析显示文本。

### 配置源超出限制

读取 `budget_error()`，并用 `source_budget_id()` 定位拒绝扣费的局部或聚合作用域。拆分配置源只有在每个外层组合配置源仍能容纳总输入时，才可能缓解子配置源的局部累计限制；它不会重置祖先预算，也不能解决嵌套深度等所有资源维度。聚合限额超限时，应减少总输入，或在评估输入边界后调整该组合配置源的对应限额。拆分还会增加子配置源数量。例如，两个各含 4 个 UTF-8 字节的配置源，在输入字节上限为 6 的组合配置源中仍累计消耗 8 字节，不会因拆层而通过。

### 合并后配置没有改变

先使用 `Config::from_source(&source)` 独立检查配置源结果，再查看其配置键。该入口使用与合并相同的库管理加载路径，即使自定义配置源覆盖了 `load()` 也是如此。配置源加载失败或事务式合并失败时，目标配置保持不变。目标中的 final 配置项也会拒绝后续覆盖。

## 限制与最佳实践

- 默认 feature 集为空；格式和富类型支持必须有意识地启用。
- 配置键和 section 路径会被校验；不要依赖普通配置键的隐式去空白或规范化。
- 普通读取不会插值。把插值放在显式调用点，让信任边界清晰可见。
- 只有选择 `InterpolationSources::ConfigThenEnv` 后才会回退到进程环境变量。
- 内置配置源默认同时支持局部边界与组合配置源的聚合边界。TOML/YAML 会在 AST 构建前检查
  parser token 和 parser depth；node、property 和语义深度仍在 flatten 阶段检查。解析准入不限制
  单个 scanner token、CPU 时间或准入后 AST 的成本。
  JSON wire 使用独立的 `ConfigWireLimits`；需要原始输入准入时必须使用有限制的 slice API。
- 配置源层独立创建并事务式合并，但 `Config` 本身是可变的；应用需要自行决定所有权和同步方式。
- `ConfigReader` 是封闭（sealed）且不满足对象安全要求，因为它含有泛型方法。请使用 `&impl ConfigReader` 等泛型约束，而不是 `dyn ConfigReader`。
- `ConfigSection` 是借用视图。使用期间必须保持来源 `Config` 存活，并在 section 内使用相对配置键。
- 将 `Debug` 输出视为诊断视图：值会被遮盖，它不是序列化格式。

## 延伸阅读

- [English README](../README.md)
- [中文 README](../README.zh_CN.md)
- [English user guide](user_guide.md)
- [English design document](design.md)
- [中文设计说明](design.zh_CN.md)
- [docs.rs API 文档](https://docs.rs/qubit-config)
- [仓库](https://github.com/qubit-ltd/rs-config)
