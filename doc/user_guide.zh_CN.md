# qubit-config 用户手册

[English](user_guide.md) | 简体中文

本手册针对 `qubit-config` `0.16.0`。读者是需要从多个来源加载配置、以类型化方式读取配置，并在输入无效时获得可诊断错误的 Rust 应用开发者。本手册不要求业务代码绑定到某一种文件格式。

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
qubit-config = "0.16"
```

默认 feature 集为空。按需显式添加可选能力：

```toml
# TOML 与 .env 配置源
qubit-config = { version = "0.16", features = ["toml", "env-file"] }
```

```toml
# Chrono 与 URL 值
qubit-config = { version = "0.16", features = ["chrono", "url"] }
```

```toml
# 所有可选值类型与格式配置源
qubit-config = { version = "0.16", features = ["full"] }
```

原子可选 feature 为 `bigdecimal`、`chrono`、`num-bigint`、`url`、`env-file`、`toml` 和 `yaml`。`rich-types` 组合前四个富类型 feature；`formats` 组合三个格式 feature；`full` 同时启用两组 feature。

结构化反序列化和 JSON 持久化示例还需要直接添加 Serde 依赖：

```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

最小的内存配置如下：

```rust
use qubit_config::Config;

let mut config = Config::new();
config.set("server.port", 8080)?;
let port: u16 = config.get("server.port")?;
# Ok::<(), qubit_config::ConfigError>(())
```

## 核心工作流

### 写入和读取值

配置键必须是规范的非空点分名称。`server` 和 `server.port` 合法；`.server`、`server.` 和 `server..port` 会被拒绝，不会自动去除空白或规范化。

```rust
use qubit_config::{Config, ConfigError};

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
# Ok::<(), ConfigError>(())
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

let mut config = Config::new();
config.set("server.host", "localhost")?;
config.set("server.port", 8080i32)?;
assert_eq!(read_endpoint(&config)?, ("localhost".to_owned(), 8080));
# Ok::<(), qubit_config::ConfigError>(())
```

`ConfigSection` 是严格相对的，可以继续嵌套：

```rust
# use qubit_config::{Config, ConfigReader};
# let mut config = Config::new();
# config.set("server.tls.enabled", true)?;
let server = config.section("server")?;
let tls = server.section("tls")?;
let enabled: bool = tls.get("enabled")?;
# let _ = enabled;
# Ok::<(), qubit_config::ConfigError>(())
```

判断点分 section 是否存在时使用 `contains_section("server.tls")`。只有明确需要原始字符前缀匹配时才使用 `contains_key_prefix("server")`；它也可能匹配 `server2` 等同名前缀。路径敏感的 `section`、`contains`、`get_property`、`is_unset`、`remove` 和 `ConfigReader::resolve_key` 都返回 `ConfigResult`，因为非法路径是可观察错误。

### 加载和合并配置源

每次调用 `ConfigSource::load` 都会生成独立配置层。自定义配置源实现
`load_into(&mut SourceLoadContext)`，必须把 context 作为唯一输出入口。
默认的 `ConfigSource::load` 方法会通过标准预算和事务配置层执行器加载配置源。
每次赋值应调用 `context.set(...)`；外部输入和解析工作应在执行前通过对应的记账方法
报告。框架无法从最终配置层推断遗漏的外部 I/O 或解析工作。内置配置源包括：

- 始终可用的 `PropertiesConfigSource`，支持 `.properties` 文件或内存内容；
- 始终可用的 `EnvConfigSource`，读取进程环境变量；
- 由 `toml` feature 启用的 `TomlConfigSource`；
- 由 `yaml` feature 启用的 `YamlConfigSource`；
- 由 `env-file` feature 启用的 `EnvFileConfigSource`；
- 按顺序合并其他配置源的 `CompositeConfigSource`。

properties 解析器遵循 Java properties 的转义规则。它会解码
`\t`、`\n`、`\r`、`\f`、被转义的分隔符和空格、合法的 `\uXXXX`
UTF-16 code unit 以及合法的 surrogate pair。像 `\u12G4` 这样错误或
不完整的 Unicode 转义会原样保留；未知的非 Unicode 转义会按
Java properties 行为去掉前导反斜杠。

`EnvFileConfigSource` 会把 `$NAME` 和 `${NAME}` 占位符作为字面值保留。应
通过显式的 `*_interpolated` 读取策略在后续读取时解析；加载 `.env` 文件不会
隐式读取进程环境变量。YAML anchor 和 alias 会由预扫描拒绝；预扫描会跳过引号、
注释和块标量内容，因此 alias 展开不会放大物化后的配置。所有内置配置源都通过
`SourceLoadContext` 加载；组合配置源会同时检查子配置源的局部策略与共享聚合策略，
全部通过后才扣减。TOML 和 YAML 的节点、配置项数量与嵌套深度只能在解析器已经物化
AST 后的展平阶段记账，因此它们约束可接受的配置结果，不限制解析器中间阶段的内存
分配或递归深度。

如果不需要在加载前定制目标配置，可以使用便捷构造函数：

```rust,no_run
use qubit_config::Config;

let config = Config::from_properties_file("config.properties")?;
# let _ = config;
# Ok::<(), qubit_config::ConfigError>(())
```

如果目标已有值或读取策略，则使用 `merge_properties_from_source`：

```rust
use qubit_config::source::{
    CompositeConfigSource, PropertiesConfigSource,
};
use qubit_config::Config;

let mut source = CompositeConfigSource::new();
source.add(PropertiesConfigSource::from_content("port=8080\n"));
source.add(PropertiesConfigSource::from_content("port=9090\n"));

let mut config = Config::new();
config.merge_properties_from_source(&source)?;
assert_eq!(config.get::<i64>("port")?, 9090);
# Ok::<(), qubit_config::ConfigError>(())
```

配置源加载和合并在公共边界具有事务语义：配置源先加载到独立配置层，完整校验输入配置层后才合并；失败时目标配置保持不变。final 配置项拒绝后续覆盖，并返回 `PropertyIsFinal`。

### 反序列化结构化值

`Config::deserialize` 会把精确 property 或点分 subtree 映射为由 Serde 管理的类型。空 prefix 表示根 map。

```rust
use qubit_config::Config;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
}

let mut config = Config::new();
config.set("server.host", "localhost")?;
config.set("server.port", 8080)?;

let server: Server = config.deserialize("server")?;
assert_eq!(server.port, 8080);
# Ok::<(), qubit_config::ConfigError>(())
```

如果通过泛型 `ConfigReader` 或 section 调用 `deserialize`、`deserialize_interpolated`、`deserialize_lenient` 或 `deserialize_interpolated_lenient`，请导入 `ConfigSerdeExt`。结构化读取默认严格，目标 `Deserialize` 类型未消费的字段会返回 `UnknownProperties`；只有明确允许额外字段时才使用 lenient 版本。struct 字段、`serde(rename)`、`serde(alias)`、`serde(default)`、嵌套 struct、map 和 `serde(flatten)` 共同声明可接受的配置形状。结构化读取会保留配置查找和转换上下文；仅由 Serde 报告的结构不匹配会变成已脱敏的 `DeserializeError`。

### 持久化和解码配置

`Config` 通过稳定的、带版本的 V1 JSON wire 格式实现 Serde 序列化；解码时也继续接受旧的无版本载荷。面对完整的不可信输入时，可以使用带默认结构限制的解码器：

```rust
use qubit_config::Config;

let mut config = Config::new();
config.set("server.port", 8080)?;
let bytes = config.encode_json_vec()?;
let restored = Config::decode_json_slice(&bytes)?;
assert_eq!(restored.get::<i64>("server.port")?, 8080);
# Ok::<(), Box<dyn std::error::Error>>(())
```

当默认限制配置不适用时，`Config::encode_json_vec_with_limits` 和
`Config::decode_json_slice_with_limits` 都接收 `ConfigWireLimits`。该
限制配置组合 rs-budget 的定向 `JsonDecodeLimits`、`JsonEncodeLimits` 与配置专属的
配置项、配置项键限制。输入通过一个 `JsonDecodeSession` 准入，输出结构和字节
通过一个 `JsonEncodeSession` 记账；配置专属限制仍由本 crate 处理。
普通 Serde 序列化的行为保持不变。该 JSON 预算与读取文本配置源时
使用的 `SourceLimits` 相互独立。普通 `Deserialize` 会用
`ConfigWireLimits::default()` 创建有预算约束的已解码值 visitor，从而限制已解码结构、
载荷、配置项数量和配置项键。通用 Serde 反序列化器不提供原始字节流或 JSON 词法
token，因此普通 `Deserialize` 无法执行原始输入限制。不可信 JSON 应使用
`Config::decode_json_slice` 或其自定义限制版本。

## 进阶用法

### 选择读取策略

`Config` 为直接读取持有默认 `ReadPolicy`。`read_with` 创建临时借用视图，在不修改配置的情况下使用另一套 policy：

```rust
use qubit_config::{Config, ConfigReader};
use qubit_config::options::ReadPolicy;

let mut config = Config::new();
config.set("HTTP_ENABLED", "yes")?;
config.set("HTTP_PORTS", "8080, 8081,,8082")?;

let policy = ReadPolicy::env_friendly();
let reader = config.read_with(&policy);
let enabled: bool = reader.get("HTTP_ENABLED")?;
let ports: Vec<u16> = reader.get("HTTP_PORTS")?;
assert!(enabled);
assert_eq!(ports, [8080, 8081, 8082]);
# Ok::<(), qubit_config::ConfigError>(())
```

`ReadPolicy` 组合控制字符串空白处理、布尔字面量、集合拆分、数值转换、Duration 转换和插值限制。应用可以使用 builder 方法选择更严格或不同的策略。传入完整的 `StringConversionPolicy` 或 `CollectionConversionPolicy` 时，该嵌套策略会原样替换；叶级 `blank_string_policy` 和 `empty_item_policy` setter 只修改各自对应的嵌套设置。底层转换选项类型由 `qubit-datatype` 提供；如果应用直接配置这些低层选项，也应直接依赖该 crate。

### 显式执行插值

普通的 `get` 和 `deserialize` 会原样保留 `${host}` 这样的占位符。只有当插值是配置契约的一部分时，才使用对应的 interpolated 方法：

<!-- example: config_interpolation -->
```rust
use qubit_config::{Config, ConfigReader};

let mut config = Config::new();
config.set("host", "localhost")?;
config.set("url", "http://${host}")?;

assert_eq!(config.get::<String>("url")?, "http://${host}");
assert_eq!(config.get_interpolated::<String>("url")?, "http://localhost");
# Ok::<(), qubit_config::ConfigError>(())
```

默认插值来源是 `ConfigOnly`。如果要回退查询进程环境变量，必须显式配置：

```rust
use qubit_config::{Config, ConfigReader};
use qubit_config::options::{InterpolationSources, ReadPolicy};

let policy = ReadPolicy::builder_from(&ReadPolicy::env_friendly())
    .interpolation_sources(InterpolationSources::ConfigThenEnv)
    .build();
let config = Config::builder().default_read_policy(policy).build();
```

能够选择环境变量名称的配置应被视为受信任输入。默认插值限制为 64 层引用链深度、
每次读取 4,096 次占位符展开，以及 1 MiB UTF-8 输出。可以通过
`ReadPolicy::builder()` 分别调整；超限会返回结构化 `ConfigError` 类别。

### 规范化环境变量配置键

`EnvConfigSource::from_prefix("APP_")` 依次执行：

1. 选择以 `APP_` 开头的名称；
2. 去除前缀；
3. 将剩余名称转为小写；
4. 把 `__` 转换为 `.`，单个 `_` 保留。

例如，`APP_DATABASE__MAX_CONNECTIONS` 会变成 `database.max_connections`。如果只需要部分转换，可以使用 `EnvConfigOptions`。如果两个不同的环境变量名称经过规范化后变成同一个配置键，加载会返回 `KeyConflict`，不会静默选择其中一个；错误会按字典序报告冲突名称，因此诊断信息不依赖操作系统的环境变量遍历顺序。

TOML 和 YAML 配置源会把映射展开成点分配置项，只接受标量、空序列和同类型标量序列。
对象数组、嵌套数组以及 YAML 异构标量序列会携带配置源、路径和索引上下文被拒绝，
不会通过隐式字符串化掩盖结构不匹配。

### 配置源限制

默认 `SourceLimits` 如下：

| 限制 | 默认值 |
| --- | ---: |
| 输入字节数 | 8 MiB（`8 * 1024 * 1024`） |
| 输出 assignment 数 | 65,536 |
| 解析结构节点数 | 262,144 |
| 组合配置源的子项数 | 256 |
| 嵌套深度 | 64 |

所有内置配置源的文件入口和内存入口使用同一套 session API。
`CompositeConfigSource::builder().limits(...).build()` 会在每个子配置源的局部策略之外增加共享聚合
策略；只有所有适用预算都接受一次累计扣减后才会统一记账。TOML 和 YAML 的节点、
配置项与深度检查发生在 AST 已经构建之后，不能限制解析器自身的内存或递归。
`SourceLimits::unbounded()` 会关闭所有配置源维度；只有应用完全控制输入边界时才应使用。

## 错误与诊断

`ConfigResult<T>` 是 `Result<T, ConfigError>`。`ConfigError` 是 non-exhaustive，下游应使用稳定类别和上下文访问器：

```rust
use qubit_config::{Config, ConfigErrorKind, ConfigReader};

let config = Config::new();
let error = config.get::<u16>("server.port").unwrap_err();

assert_eq!(error.kind(), ConfigErrorKind::PropertyNotFound);
assert_eq!(error.path(), Some("server.port"));
# let _ = ConfigErrorKind::PropertyNotFound;
```

常见类别包括 `InvalidKey`、`InvalidPath`、`PropertyNotFound`、`PropertyHasNoValue`、`TypeMismatch`、`Conversion`、`Substitution`、`SubstitutionCycle`、`SourceLimitExceeded`、`KeyConflict`、`Merge`、`PropertyIsFinal`、`Io`、`Parse` 和 `Deserialize`。

对于配置源的 IO、解析和限制错误，`source_id()` 会返回文件路径或稳定的配置源
标识；其他错误返回 `None`。
配置源预算失败时，`source_budget_id()` 会指出拒绝扣减的局部或聚合预算，
`budget_error()` 则返回结构化的 rs-budget 错误。

`get_any` 失败时可以使用 `candidate_paths()`，因为多候选键错误可能包含按查找顺序排列的多个路径。如果集合转换错误对应原始集合中的某个元素，可以使用 `source_index()` 获取其位置。

程序逻辑应使用 `kind()`、`path()` 和候选路径处理诊断。配置值的 `Debug` 输出会通过 `qubit-redact` 遮盖存储值，同时保留 property 元数据。

## 排障

### 默认值没有生效

检查配置键是否存在并且有值。默认值只适用于缺失或实际缺失的配置键；如果配置键存在但无法转换为目标类型，`get_or` 会立即返回 `ConversionError`。

### `${...}` 没有被替换

使用 `get_interpolated`、`get_interpolated_or`、`get_any_interpolated` 或 `deserialize_interpolated`。普通读取会有意保留占位符字面量。如果占位符应来自进程环境变量，请确认当前 policy 使用了 `ConfigThenEnv`。

### section 无法读取配置键

检查 section 作用域和相对配置键。`config.section("server")?.get("port")` 解析为 `server.port`；section 不包含它自身的根标量。使用 `contains_section` 判断 section 成员，使用 `keys()` 查看当前可见的相对配置键。

### 文件 loader 不可用

检查 `Cargo.toml` 中的 feature：`toml` 启用 `TomlConfigSource`，`yaml` 启用 `YamlConfigSource`，`env-file` 启用 `EnvFileConfigSource`。`PropertiesConfigSource` 和 `EnvConfigSource` 不需要这些格式 feature。

### 配置源报告键冲突

检查环境变量规范化后的名称或结构化配置键展平结果。去前缀、转小写和下划线转换可能使不同名称合并为同一个配置键；TOML/YAML 文档也可能产生重复的展平配置键。请重命名输入，或减少规范化步骤。

### 配置源超出限制

读取 `budget_error()` 中的 `SourceLimitKind`，并与对应的 `SourceLimits` 维度对照；
`source_budget_id()` 可以区分子配置源局部限制与组合配置源聚合限制。只有明确了解
输入边界时才增加单项限制，也可以把输入拆成更小的配置源层。

### 合并后配置没有改变

先使用 `source.load()` 独立检查配置源结果，再查看其配置键。配置源加载失败或事务式合并失败时，目标配置保持不变。目标中的 final 配置项也会拒绝后续覆盖。

## 限制与最佳实践

- 默认 feature 集为空；格式和富类型支持必须有意识地启用。
- 配置键和 section 路径会被校验；不要依赖普通配置键的隐式去空白或规范化。
- 普通读取不会插值。把插值放在显式调用点，让信任边界清晰可见。
- 只有选择 `InterpolationSources::ConfigThenEnv` 后才会回退到进程环境变量。
- 内置配置源默认同时支持局部边界与组合配置源的聚合边界。TOML/YAML 的节点、
  配置项和深度记账在第三方 AST 物化后才开始，不能保护解析器自身的内存分配或递归。
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
