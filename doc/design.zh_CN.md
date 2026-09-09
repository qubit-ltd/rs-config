# qubit-config 设计说明

[English](design.md) | 简体中文

本文描述 `qubit-config` `0.17.0` 已实现的架构。单个调用及其边界行为仍以公共
API 文档和测试为准。

## 稳定核心

当前兼容性核心包括 `Config`、`ConfigReader`、`ConfigSection`、`ReadPolicy`
和 `ConfigSerdeExt`：

- `Config` 持有使用规范点分键的配置项，以及运行时默认读取策略。
- `ConfigReader` 为 `Config` 和 `ConfigSection` 提供类型化、可选、默认值、
  多候选键、列表、严格类型和插值读取。
- `ConfigSection` 是借用视图，其中的配置键严格相对于点分路径；与 section
  路径同名的标量不属于它的子项。
- `ReadPolicy` 统一管理转换语义、转换限制、插值来源和插值限制。
- `ConfigSerdeExt` 将一个精确配置项或配置子树投影为由 Serde 管理的类型。

`ConfigReader` 是封闭（sealed）trait，因为它的默认方法依赖 crate 内部维护的
不变量。它包含泛型方法，因此也不满足对象安全要求。下游应使用
`R: ConfigReader + ?Sized` 或 `&impl ConfigReader` 等泛型约束，不应实现第三方
`ConfigReader`，也不应使用 `dyn ConfigReader`。

单键参数使用 `AsRef<str>`，支持借用/owned 字符串和 `ConfigKey`。键校验仍在配置边界执行，
参数 trait 不证明键有效。多候选键的 `ConfigNames` 独立保留。默认值直接使用
`qubit_value::IntoValueDefault`，仅在需要回退时执行转换；不保留 `ConfigName` 或
`IntoConfigDefault` 的公共兼容别名。

## 配置源管线

`ConfigSource::load` 是公共执行边界。它创建新的 `SourceLoadContext`，调用
`load_into`，并且仅在加载成功后返回上下文持有的独立 `Config` 配置层。
自定义配置源通过 `SourceLoadContext::set` 或 `set_null` 写入配置，并在消耗资源前
报告输入字节、解析节点和子配置源数量。框架无法根据最终配置层推断自定义配置源
没有报告的外部 I/O 或解析工作。

`PropertiesConfigSource`、`EnvConfigSource` 和 `CompositeConfigSource` 始终可用；
TOML、YAML 和 `.env` 适配器由 feature 控制。组合配置源按照添加顺序加载子项，
再将每个完整子配置层合并到自身。后加载的值会覆盖先前值，但已有配置项标记为
final 时拒绝覆盖。公共边界提供事务语义：失败不会暴露不完整的配置源结果，也不会
对调用方的目标配置产生部分修改。

## 读取策略与 Section

`Config` 在运行时持有默认 `ReadPolicy`。`read_with` 使用临时借用策略创建读取视图，
嵌套 section 会继承该覆盖策略。运行时策略不参与持久化数据的相等性比较，也不属于
V1 wire 表示。

只有配置项不存在，或根据当前字符串策略被视为实际缺失时，才会采用默认值；转换
失败不会被默认值掩盖。显式空集合仍被视为已存在。多候选键读取按照调用方提供的
顺序查找，section 则只解析其路径下的相对配置键。

`Config::get` 仍是按策略转换的读取。来自值层的缺失错误保留 `ValueMissing`、原始转换来源
和集合索引。集合某项非法或被策略判定为缺失时，默认值和候选键搜索都必须停止。
具体空集合的首项也不能回退。`PropertyHasNoValue` 仅用于配置 reader 的策略预检查，
不再替代值错误而丢失上下文。

普通类型化读取各自使用独立的转换操作。一次结构化 Serde 物化会让所有字段、映射、
序列、枚举变体和嵌套值共享一个 `ConversionSession`，因此操作限制会在整个结果内
累计。

## 插值

普通 `get` 和 `deserialize` 调用会保留 `${name}` 字面量，只有
普通读取的 `*_interpolated` API 或 `deserialize_with` 的 `interpolate` 选项才会执行插值。
带作用域的读取会先查当前 reader，再查根配置。
仅当策略显式选择 `InterpolationSources::ConfigThenEnv` 时才会继续查询进程环境变量；
`ReadPolicy::env_friendly()` 只改变转换行为，不会开启该回退。

默认限制为 64 层引用链深度、每次读取 4,096 次占位符展开，以及 1 MiB UTF-8 输出。
循环引用和各类资源耗尽都有结构化 `ConfigError` 类别。允许配置内容选择环境变量名称
会形成信任边界。加载 `.env` 时会保留 `$NAME` 和 `${NAME}` 字面量，不会隐式读取
进程环境变量。

## 结构化 Serde

`ConfigSerdeExt` 会优先选择精确配置项；如果不存在，则根据请求前缀下的子项构造
JSON 风格对象。同一前缀同时存在精确配置项和子项时返回 `KeyConflict`。空前缀表示
reader 当前可见的全部配置项。

结构化读取默认拒绝目标类型未消费的配置项，并通过
`ConfigError::UnknownProperties` 报告经过排序的根相对路径。对于明确开放的配置区域，
应选择 `UnknownFieldPolicy::Ignore`。`deserialize(prefix)` 默认不插值并拒绝未知字段；
`deserialize_with(prefix, ConfigDeserializeOptions)` 可设置 `interpolate` 和
`unknown_fields: UnknownFieldPolicy::{Reject, Ignore}`，结构化读取只保留这两个入口。
查找、插值和转换错误会保留配置上下文；只有 Serde
报告的结构不匹配才会转换为已脱敏的 `DeserializeError`。投影遵循 Serde 的 JSON
风格数据模型，不承诺覆盖 `ConfigReader::get` 为富类型提供的所有原生转换形态。

准备树保存借用值、集合 slice、JSON 节点和合成对象索引，不克隆完整 JSON 中间载荷。
两个对象贡献不同叶子时可以合并；重复叶子和标量/对象冲突会失败。插值覆盖使用带类型的源位置，
JSON 键中的字面点号不会与嵌套路径混淆。只展开真正的字符串叶子，每个叶子处理一次；
URL 等富类型格式化后的文本不作为插值输入，未变化文本保持借用。

执行 visitor 前，对完整选中输入准入，包括目标忽略的字段。插值读取分别检查原始输入和展开后
输入。这些检查取 reader 的转换限制，但计数器与叶子共享的 `ConversionSession` 独立。
明确的数值目标从原始运行时类型转换；`deserialize_any` 遵循自然 JSON 类别，包括宽整数的
十进制文本表示。标量列表拆分复用已准入元素，真实集合内的字符串不再拆分。
精确标量被策略判定缺失时可读取为 `None`，子树中的缺失标量字符串会被省略，集合某项缺失
仍然报错。公共返回值仍要求 `DeserializeOwned`。

## Wire 持久化

Serde 序列化会输出确定性的 V1 JSON 封装，其中包含显式 `version` 字段，配置键按
字典序排列。解码同时接受 V1 和旧版无版本顶层表示，会校验规范映射键与配置项内部
名称，并拒绝未知版本。运行时读取策略有意不进入 V1 持久化契约；旧表示中的策略数据
可以读入，但不会应用。

`Config::encode_json_vec` 和 `Config::decode_json_slice` 使用默认的有限 wire 配置；
对应的 `*_with_limits` 变体接收自定义 `ConfigWireLimits`。普通 Serde
`Deserialize` 可以限制已解码值，却无法对原始输入字节或 JSON 词法 token 做准入；
完整的不可信 JSON 输入必须使用 `decode_json_slice`。

持久化序列化与 `ConfigSerdeExt` 读取业务结构体是不同能力。本次不增加业务结构体反向展平
写入配置的 serializer，现有 Wire V1 表示保持不变。

## 资源预算

crate 为不同操作使用相互独立的资源域，而不是共享一个计数器：

| 资源域 | 默认限制 | 作用边界 |
| --- | --- | --- |
| 类型转换 | `qubit-datatype` 的 `ConversionLimits::default()` | 单次普通读取，或一次完整的结构化物化 |
| 结构化输入准入 | reader 的转换操作与结构值限制 | 完整选中输入；插值前后分别检查，计数独立 |
| 插值 | 深度 64；展开 4,096 次；输出 1 MiB | 单次插值读取 |
| 配置源加载 | 输入 8 MiB；赋值 65,536 个；节点 262,144 个；组合子配置源 256 个；深度 64 | 单个配置源及其所有外层组合配置源作用域 |
| JSON wire | 输入/输出 1 MiB；深度 64；节点 100,000 个；序列项、映射项和配置项各 4,096 个；字符串和对象键 256 KiB；数字 4 KiB；载荷 1 MiB；配置项键 256 字节 | 单次编码或解码 |

配置源的字节、赋值、节点和子配置源计数是累计预算，只有全部适用的局部与聚合
作用域都接受一次扣减时才会共同提交；深度是点限制。TOML 和 YAML 会先由第三方解析器
构建 AST，再进行展平，因此节点、赋值和深度预算约束的是最终可接受的配置，
无法限制解析器自身的内存分配或递归。Wire 预算与 `SourceLimits`、读取转换限制互相
独立。

## Feature 边界

默认 feature 集为空。核心存储、reader、配置项、`.properties`、进程环境变量和组合
配置源不依赖可选 feature。

| Feature | 边界 |
| --- | --- |
| `bigdecimal`、`chrono`、`num-bigint`、`url` | 启用对应富类型及其转换支持 |
| `env-file`、`toml`、`yaml` | 启用对应格式配置源和便捷构造函数 |
| `rich-types` | 启用全部四个富类型 feature |
| `formats` | 启用全部三个可选格式配置源 |
| `full` | 同时启用 `rich-types` 和 `formats` |

应用应只启用实际使用的能力；如果应用代码直接导入 `serde`、`serde_json` 或
`qubit-datatype` 的 API，也应直接声明相应依赖。

## 下游兼容承诺

下游可以依赖公共类型化读取组合、section 的严格相对语义、按调用顺序尝试候选键、
配置源顺序与 final 值保护、显式插值、稳定错误类别与上下文访问器、已声明的 feature
边界，以及 V1 wire 与旧版无版本表示的解码能力。

兼容承诺不包括私有模块布局、错误消息的精确显示文本、第三方 `ConfigReader` 实现、
`dyn ConfigReader`、隐式环境变量插值、异步加载、热加载编排或模式定义 DSL。错误枚举
是 non-exhaustive；下游逻辑应根据 `ConfigErrorKind` 和 `path()`、
`candidate_paths()`、`source_id()`、`source_budget_id()`、`budget_error()` 等上下文
访问器进行分支处理。

受支持的使用方式请参阅[用户手册](user_guide.zh_CN.md)、
[API 文档](https://docs.rs/qubit-config)和[项目 README](../README.zh_CN.md)。
