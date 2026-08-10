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
| `Config` | 持有配置属性，提供修改、source 合并、序列化和根 reader scope。 |
| `Property` | 保存一个规范 key、标量或集合值容器、可选描述和 final 标记。 |
| `ValueContainer` | 保留 source 提供的是标量还是显式集合；集合元素会分别转换。 |
| `ConfigReader` | 由 `Config` 和 `ConfigSection` 实现的 sealed 只读接口，提供类型化、可选、默认值、多 key、列表和严格读取。 |
| `ConfigSection` | 在点分路径下提供严格相对 key 的借用 reader 视图。 |
| `ReadPolicy` | 控制字符串、布尔值、集合、数值、Duration 和插值读取行为。 |
| `ConfigSource` | 产生独立的 `Config` layer，可检查，也可合并到目标配置。 |
| `CompositeConfigSource` | 按添加顺序加载多个 source；同一个 key 的后加载 layer 覆盖前一个，除非 property 是 final。 |

需要区分两种转换方式：

- `ConfigReader` 的 `get::<T>` 等方法通过 `FromConfig` 把一个存储的 property 转换为目标类型。
- `ConfigSerdeExt` 和 `Config::deserialize` 把一个精确 property 或 subtree 投影为由 Serde 管理的类型。

根 `Config` 和 section 共享相同的读取方法，但 key scope 不同。在 `server` 创建的 section 中，`host` 会解析为 `server.host`；它不会把恰好名为 `server` 的标量 property 暴露为子项。

### 公共 API 分层

当前建议作为稳定核心依赖的 API 是 `Config`、`ConfigReader`、
`ConfigSection`、`ReadPolicy` 和 `ConfigSerdeExt`。这些类型覆盖配置持有、
类型化读取、作用域视图、转换策略和结构化反序列化。

`PropertiesConfigSource`、`EnvConfigSource`、`TomlConfigSource` 和
`YamlConfigSource` 等 source adapter 属于独立的加载层，应按应用实际需要的
输入格式选择和启用。持久化与 wire 解码，以及低层 `Property` 操作，属于
需要具体能力时再使用的外围 API。当前稳定核心不包含 reload framework、
异步 source、object-safe reader 或 schema DSL。

## 贯穿场景：基础配置叠加部署覆盖

假设服务有如下本地基础配置：

```properties
server.host=localhost
server.port=8080
server.timeout=30
```

部署环境可以设置 `APP_SERVER__HOST` 和 `APP_SERVER__PORT`。成功标准是：

- 没有覆盖值时，本地配置仍然可用；
- 环境变量只覆盖同名 key；
- 应用通过相对 section 读取 `server`；
- 格式错误的值返回错误，而不是静默选择默认值。

下面的示例先使用内存中的 `.properties` layer 以保证确定性，再加入进程环境变量 layer。`EnvConfigSource::with_prefix` 会选择 `APP_`，去除此前缀，将剩余部分转成小写，并把双下划线转换为点号，单个下划线保留。

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
    sources.add(EnvConfigSource::with_prefix("APP_"));

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

运行应用时设置 `APP_SERVER__PORT=9090`，即可观察到环境变量覆盖。只存在于 properties layer 的 key 仍然可见。如果设置 `APP_SERVER__PORT=not-a-number`，`server.get::<u16>("port")` 会返回转换错误；`get_or` 不会掩盖已存在但无效的值。

## 安装与最小配置

crate 要求 Rust `1.94` 或更高版本，并使用 edition `2024`。

```toml
[dependencies]
qubit-config = "0.16"
```

默认 feature 集为空。按需显式添加可选能力：

```toml
# TOML 与 .env source
qubit-config = { version = "0.16", features = ["toml", "env-file"] }
```

```toml
# Chrono 与 URL 值
qubit-config = { version = "0.16", features = ["chrono", "url"] }
```

```toml
# 所有可选值类型与格式 source
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

key 必须是规范的非空点分名称。`server` 和 `server.port` 合法；`.server`、`server.` 和 `server..port` 会被拒绝，不会自动 trim 或规范化。

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
```

缺失值的关键规则如下：

- key 缺失时，可以使用 `get_optional`、`get_or` 及其多 key 变体；
- unset property 或标量空白字符串在当前 `ReadPolicy` 下可能被视为 effectively missing；
- 显式空集合仍然存在，因此 `get_optional_list` 返回 `Some(Vec::new())`；
- 已存在但无法转换的值会立即返回错误。

需要候选名称时，使用 `get_any`、`get_optional_any` 或 `get_any_or`。它们按照传入顺序检查 key，并选择第一个不被视为缺失的值。

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
let server = config.section("server")?;
let tls = server.section("tls")?;
let enabled: bool = tls.get("enabled")?;
# let _ = enabled;
```

判断点分 section 是否存在时使用 `contains_section("server.tls")`。只有明确需要原始字符前缀匹配时才使用 `contains_key_prefix("server")`；它也可能匹配 `server2` 等同名前缀。path-sensitive 的 `section`、`contains`、`get_property`、`is_unset`、`remove` 和 `ConfigReader::resolve_key` 都返回 `ConfigResult`，因为非法路径是可观察错误。

### 加载和合并 source

每次调用 `ConfigSource::load` 都会生成独立 layer。内置 source 包括：

- 始终可用的 `PropertiesConfigSource`，支持 `.properties` 文件或内存内容；
- 始终可用的 `EnvConfigSource`，读取进程环境变量；
- 由 `toml` feature 启用的 `TomlConfigSource`；
- 由 `yaml` feature 启用的 `YamlConfigSource`；
- 由 `env-file` feature 启用的 `EnvFileConfigSource`；
- 按顺序合并其他 source 的 `CompositeConfigSource`。

properties parser 遵循 Java properties 的 escape dialect。它会解码
`\t`、`\n`、`\r`、`\f`、被转义的分隔符和空格、合法的 `\uXXXX`
UTF-16 code unit 以及合法的 surrogate pair。像 `\u12G4` 这样错误或
不完整的 Unicode escape 会原样保留；未知的非 Unicode escape 会按
Java properties 行为去掉前导反斜杠。

`EnvFileConfigSource` 会把 `$NAME` 和 `${NAME}` 占位符作为字面值保留。应
通过显式的 `*_interpolated` 读取策略在后续读取时解析；加载 `.env` 文件不会
隐式读取进程环境变量。YAML anchor 和 alias 会由预扫描拒绝，预扫描会跳过
引号、注释和 block scalar 内容，因此 alias 展开不会放大物化后的配置。输入
字节数限制在 parser 前检查。对于 TOML 和 YAML，property 数量与嵌套深度限制
在 parser 物化 AST 后的 flatten 阶段执行；它们约束最终配置，不限制 parser
中间阶段的内存分配或递归深度。

如果不需要在加载前定制目标配置，可以使用便捷构造函数：

```rust
let config = Config::from_properties_file("config.properties")?;
# let _ = config;
```

如果目标已有值或读取策略，则使用 `merge_properties_from_source`：

```rust
use qubit_config::source::{
    CompositeConfigSource, PropertiesConfigSource,
};

let mut source = CompositeConfigSource::new();
source.add(PropertiesConfigSource::from_content("port=8080\n"));
source.add(PropertiesConfigSource::from_content("port=9090\n"));

let mut config = Config::new();
config.merge_properties_from_source(&source)?;
assert_eq!(config.get::<i64>("port")?, 9090);
# Ok::<(), qubit_config::ConfigError>(())
```

source 加载和合并在公共边界具有事务语义：source 先加载到独立 layer，完整校验输入 layer 后才合并；失败时目标配置保持不变。final property 拒绝后续覆盖，并返回 `PropertyIsFinal`。

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

当默认 profile 不适用时，`Config::encode_json_vec_with_limits` 和
`Config::decode_json_slice_with_limits` 都接收 `ConfigWireLimits`。该
profile 组合 rs-budget 的 `JsonLimits` 与配置专属的 property、property
key 限制。共享 JSON adapter 在一次遍历中负责 input、output、depth、node、
collection、key、string 和 number 资源；配置专属限制仍由本 crate 处理。
普通 Serde 序列化的行为保持不变。该 JSON budget 与读取文本 source 时
使用的 `SourceLimits` 相互独立。

## 进阶用法

### 选择读取策略

`Config` 为直接读取持有默认 `ReadPolicy`。`read_with` 创建临时借用视图，在不修改配置的情况下使用另一套 policy：

```rust
use qubit_config::{Config, ConfigReader};
use qubit_config::options::ReadPolicy;

let mut config = Config::new();
config.set("HTTP_ENABLED", "yes")?;
config.set("HTTP_PORTS", "8080, 8081,,8082")?;

let reader = config.read_with(&ReadPolicy::env_friendly());
let enabled: bool = reader.get("HTTP_ENABLED")?;
let ports: Vec<u16> = reader.get("HTTP_PORTS")?;
assert!(enabled);
assert_eq!(ports, [8080, 8081, 8082]);
# Ok::<(), qubit_config::ConfigError>(())
```

`ReadPolicy` 组合控制字符串空白处理、布尔字面量、集合拆分、数值转换、Duration 转换和插值限制。应用可以使用 builder 方法选择更严格或不同的策略。底层转换选项类型由 `qubit-datatype` 提供；如果应用直接配置这些低层选项，也应直接依赖该 crate。

### 显式执行插值

普通的 `get` 和 `deserialize` 会原样保留 `${host}` 这样的占位符。只有当插值是配置契约的一部分时，才使用对应的 interpolated 方法：

```rust
use qubit_config::{Config, ConfigReader};

let mut config = Config::new();
config.set("host", "localhost")?;
config.set("url", "http://${host}")?;

assert_eq!(config.get::<String>("url")?, "http://${host}");
assert_eq!(config.get_interpolated::<String>("url")?, "http://localhost");
# Ok::<(), qubit_config::ConfigError>(())
```

默认插值 source 是 `ConfigOnly`。如果要回退查询进程环境变量，必须显式配置：

```rust
use qubit_config::{Config, ConfigReader};
use qubit_config::options::{InterpolationSources, ReadPolicy};

let policy = ReadPolicy::env_friendly()
    .with_interpolation_sources(InterpolationSources::ConfigThenEnv);
let config = Config::new().with_default_read_policy(policy);
```

能够选择环境变量名称的配置应被视为受信任输入。插值还具备递归深度、展开次数和输出大小限制；超限会返回结构化 `ConfigError` 类别。

### 规范化环境变量 key

`EnvConfigSource::with_prefix("APP_")` 依次执行：

1. 选择以 `APP_` 开头的名称；
2. 去除前缀；
3. 将剩余名称转为小写；
4. 把 `__` 转换为 `.`，单个 `_` 保留。

例如，`APP_DATABASE__MAX_CONNECTIONS` 会变成 `database.max_connections`。如果只需要部分转换，可以使用 `EnvConfigOptions`。如果两个不同的环境变量名称经过规范化后变成同一个 key，加载会返回 `KeyConflict`，不会静默选择其中一个；错误会按字典序报告冲突名称，因此诊断信息不依赖操作系统的环境变量遍历顺序。

TOML 和 YAML source 会把 mapping 展开成点分隔 property，只接受标量、空 sequence 和同类型标量 sequence。对象数组、嵌套数组以及 YAML 异构标量 sequence 会带 source、path 和 index 上下文被拒绝，不会通过隐式字符串化掩盖结构不匹配。

### 配置 source limits

默认 `SourceLimits` 如下：

| 限制 | 默认值 |
| --- | ---: |
| 输入字节数 | 8 MiB（`8 * 1024 * 1024`） |
| 输出 assignment 数 | 65,536 |
| 嵌套深度 | 64 |

所有内置文本 source 对文件入口和内存入口应用同类 budget。`max_input_bytes`
在 parser 前检查。对于 TOML 和 YAML，`max_properties` 与
`max_nesting_depth` 在 parser 构建 AST 后、source flatten 期间执行；它们不是
parser 阶段的内存或递归限制。`SourceLimits::unbounded()` 会关闭这三项 source
限制；只有当输入边界由应用控制时才应使用它。

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

对于 source 的 IO、解析和限制错误，`source_id()` 会返回文件路径或稳定的
source label；其他错误返回 `None`。

`get_any` 失败时可以使用 `candidate_paths()`，因为多 key 错误可能包含按查找顺序排列的多个路径。如果集合转换错误对应原始集合中的某个元素，可以使用 `source_index()` 获取其位置。

程序逻辑应使用 `kind()`、`path()` 和候选路径处理诊断。配置值的 `Debug` 输出会通过 `qubit-redact` 遮盖存储值，同时保留 property 元数据。

## 排障

### 默认值没有生效

检查 key 是否存在并且有值。默认值只适用于缺失或 effectively missing 的 key；如果 key 存在但无法转换为目标类型，`get_or` 会立即返回 `ConversionError`。

### `${...}` 没有被替换

使用 `get_interpolated`、`get_interpolated_or`、`get_any_interpolated` 或 `deserialize_interpolated`。普通读取会有意保留占位符字面量。如果占位符应来自进程环境变量，请确认当前 policy 使用了 `ConfigThenEnv`。

### section 无法读取 key

检查 section scope 和相对 key。`config.section("server")?.get("port")` 解析为 `server.port`；section 不包含它自身的根标量。使用 `contains_section` 判断 section 成员，使用 `keys()` 查看当前可见的相对 key。

### 文件 loader 不可用

检查 `Cargo.toml` 中的 feature：`toml` 启用 `TomlConfigSource`，`yaml` 启用 `YamlConfigSource`，`env-file` 启用 `EnvFileConfigSource`。`PropertiesConfigSource` 和 `EnvConfigSource` 不需要这些格式 feature。

### source 报告 key conflict

检查环境变量规范化后的名称或结构化 key 展平结果。去前缀、转小写和下划线转换可能使不同名称合并为同一个 key；TOML/YAML 文档也可能产生重复展平 key。请重命名输入，或减少规范化步骤。

### source 超出 limit

读取错误中的 `SourceLimitKind`，并与 `SourceLimits::max_input_bytes`、`max_properties` 和 `max_nesting_depth` 对照。只有明确了解输入边界时才增加单项限制，也可以把输入拆成更小的 source layer。

### merge 后配置没有改变

先使用 `source.load()` 独立检查 source 结果，再查看其 keys。source 加载失败或事务式 merge 失败时，目标配置保持不变。目标中的 final property 也会拒绝后续覆盖。

## 限制与最佳实践

- 默认 feature 集为空；格式和富类型支持必须有意识地启用。
- 配置 key 和 section path 会被校验；不要依赖普通 key 的隐式 trim 或规范化。
- 普通读取不会插值。把插值放在显式调用点，让信任边界清晰可见。
- 只有选择 `InterpolationSources::ConfigThenEnv` 后才会回退到进程环境变量。
- 内置 source 默认受边界限制。输入字节数限制保护 parser 边界，TOML/YAML 的
  property 与深度限制保护 AST flatten；JSON wire 解码使用由 rs-budget
  `JsonBudget` 支撑的独立 `ConfigWireLimits` profile。
- source layer 独立创建并事务式合并，但 `Config` 本身是可变的；应用需要自行决定所有权和同步方式。
- `ConfigReader` 是 sealed 且不是 object-safe，因为它含有泛型方法。请使用 `&impl ConfigReader` 等泛型约束，而不是 `dyn ConfigReader`。
- `ConfigSection` 是借用视图。使用期间必须保持来源 `Config` 存活，并在 section 内使用相对 key。
- 将 `Debug` 输出视为诊断视图：值会被遮盖，它不是序列化格式。

## 延伸阅读

- [English README](../README.md)
- [中文 README](../README.zh_CN.md)
- [English user guide](user_guide.md)
- [docs.rs API 文档](https://docs.rs/qubit-config)
- [仓库](https://github.com/qubit-ltd/rs-config)
