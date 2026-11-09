# qubit-config 设计说明

[English](design.md) | 简体中文

本文描述 `qubit-config` `0.15` 已实现的架构。单个调用及其边界行为仍以公共
API 文档和测试为准。

## 稳定核心

对象成员名会以字符串形式交给 Serde 的 key deserializer；`String` 可正常读取，
数字或容器类型的 key 不会从名称文本解析。

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

库管理的配置源入口使用 crate-owned executor。它创建新的 `SourceLoadContext`，调用
`load_into`，并且仅在加载成功后返回上下文持有的独立 `Config` 配置层，同时绕过对 `load` 的覆盖。
默认 `ConfigSource::load` 使用该 executor；直接调用覆盖的 `load` 由实现者负责。
自定义配置源通过 `SourceLoadContext::set` 或 `set_null` 写入配置，并在消耗资源前
报告输入字节、解析节点和子配置源数量。框架无法根据最终配置层推断自定义配置源
没有报告的外部 I/O 或解析工作。
这些上下文方法返回的无效赋值键使用 `ConfigError::SourceInvalidKey`；`kind()` 仍为
`InvalidKey`，`path()` 保留被拒绝的键，`source_id()` 标识执行赋值的配置源（嵌套组合时为
叶子来源）。直接 `Config::set` 仍返回不带来源上下文的 `InvalidKey`。赋值校验前的预算
检查与扣费顺序保持不变。

五类文本与组合配置源 builder 提供可选 `label`。非空标签会原样用作诊断来源标识；空标签恢复
既有默认值，重复调用以最后一次为准，且与 `file`／`content` 调用顺序无关。标签不要求唯一，不能
包含秘密。它仅用于诊断，不会持久化，也不改变 Wire V1、来源顺序、final 保护或预算记账。嵌套加载中，
`source_id()` 标识正在扣费的叶子来源，`source_budget_id()` 标识拒绝扣费的局部或聚合预算层。

`PropertiesConfigSource`、`EnvConfigSource` 和 `CompositeConfigSource` 始终可用；
TOML、YAML 和 `.env` 适配器由 feature 控制。组合配置源按照添加顺序加载子项，
再将每个完整子配置层合并到自身。后加载的值会覆盖先前值，但已有配置项标记为
final 时拒绝覆盖。公共边界提供事务语义：失败不会暴露不完整的配置源结果，也不会
对调用方的目标配置产生部分修改。
TOML 和 YAML 会在构建 backend AST 前准入 parser token 并检查 parser depth，之后仍在展平时
检查原有 node、assignment 和语义输出深度。Parser depth 按单个输入检查，与展平后的语义深度
分开计算。这会限制传入 backend 的工作量，但不保证严格 RSS 或 CPU 上限。
AST 根深度为 0；每个 mapping 值和 sequence 元素增加一层，空 sequence 不增加元素深度。
自定义来源可通过 `SourceLoadContext::set_final` 将已有配置项标记为 final；它只修改元数据，
不重复扣减赋值预算，并在后续合并中遵循既有的单向 final 属性规则。
TOML 和 YAML 会在递归处理值之前拒绝空的或其他无效 mapping 成员键。合法的空 table 和
mapping 可以成功解析，但不会生成 property，也不会保留 section 的存在性：YAML `server: {}`
和 TOML `[server]` 都使 `contains_section("server")` 为 false，且
`section_if_present("server")` 返回 `None`；普通的 `section("server")` 调用仍可创建空的只读视图。
空序列仍是显式 property。合并空 mapping 或 table 不会清除已有后代配置；要求 `server` 子树的调用方
仍会把仅含空对象的来源视为缺失。需要清空集合时使用空序列。环境变量规范化冲突会按排序结果检查：选择规范化键最小的冲突组，
再选择其中原始名称最小的两个名称。对于有效且符合配置源预算的输入，该诊断结果稳定。
properties 物理行以 LF、CRLF 或 CR 结束。格式空白仅包含 ASCII 空格、TAB 和换页符；键和值中的
Unicode 空白会原样保留。仅含注释或其他无值 token 的单个 YAML 文档会加载为空层，显式根 null
仍然无效。内置文本配置源读取文件时会重试 Interrupted，且不为失败读取扣减字节预算；其他
I/O 错误保留来源路径和错误种类。
来源解析器会为 `SourceParseError` 附加可信的物理坐标，调用方可通过
`ConfigError::source_location()` 读取；无法可靠确定位置时，访问器返回 `None`。行和列均从 1 开始；
列按 Unicode scalar 值计数，TAB 计为一列。LF、CRLF 和独立 CR 都结束一个物理行。TOML 与 YAML
解析器偏移会统一换算为此坐标约定。Properties 的无效键错误只包含逻辑赋值首个物理行，不含列；
续行即使消耗空行，也仍归属首行。`source_index()` 仍是集合元素索引，不表示行号。组合加载会原样
传播叶子来源的解析错误及其位置。TOML parser guard 的 span，以及 YAML scanner 或 anchor/alias
marker 在坐标可可靠映射时会附带结构化位置；其他缺少可信来源坐标的早期 scanner 或 guard 错误仍不带
位置。预算与 I/O 错误也不附带来源位置。位置仅为结构化元数据：不会保留来源文本，也不会把 parser 输入片段加入
诊断消息或 Debug 输出。

## 读取策略与 Section

`Config` 在运行时持有默认 `ReadPolicy`。`read_with` 使用临时借用策略创建读取视图，
嵌套 section 会继承该覆盖策略。运行时策略不参与持久化数据的相等性比较，也不属于
V1 wire 表示。

只有配置项不存在，或根据当前字符串策略被视为实际缺失时，才会采用默认值；转换
失败不会被默认值掩盖。显式空集合仍被视为已存在。多候选键读取按照调用方提供的
顺序查找，section 则只解析其路径下的相对配置键。

使用 `BlankStringPolicy::TreatAsMissing` 时，普通标量空白预检查可能在启动 `FromConfig` 前直接
返回缺失或默认结果；转换的 `max_input_bytes` 不统计这次预检查。非空白值会进入转换并受该字节预算
约束。结构化反序列化则在目标 visitor 执行前检查完整输入，包括选中的 JSON 值或子树中的空白成员。
两者针对不同资源域；转换输入预算也不承诺严格的 CPU 时间上限。

`Config::get` 仍是按策略转换的读取。来自值层的缺失错误保留 `ValueMissing`、原始转换来源
和集合索引。集合某项非法或被策略判定为缺失时，默认值和候选键搜索都必须停止。
具体空集合的首项也不能回退。`PropertyHasNoValue` 仅用于配置 reader 的策略预检查，
不再替代值错误而丢失上下文。

`ConfigReader::ensure_known_keys` 是显式、可选的 schema 校验。声明的字段和开放 section
名称相对于当前 reader；报告的未知路径会解析为根配置的点分路径。该方法不会改变其他读取
入口的宽松行为，也不会检查值转换。`usize` 通过委托 `u128` 并执行受检的平台转换实现
标量 `FromConfig`。它仅是读取目标；存储类型和 Wire V1 不变，也不增加集合转换。

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
进程环境变量，注释中的引号也不会改变这一点。YAML 在 AST 构建前依据 scanner token
拒绝 anchor 和 alias；scanner 错误直接失败，而引号、注释和块标量文本不会被当作 anchor 或
alias token。

插值使用显式堆帧栈遍历引用，不再为每层引用各占一个 Rust 调用栈帧。活动帧存储会随引用链增长；
现有深度、展开次数和输出字节限制仍生效，但不承诺严格的进程内存或 CPU 上限。

展开次数会在一次逻辑读取内累计，包括多候选键搜索和结构化 visitor 最终忽略的字段。
深度按当前引用链计算，输出字节限制则分别应用于每个展开值。这些预算不会在每个候选键
或字段开始时重置。被引用值转成文本时使用当前 `ReadPolicy` 及其转换限制；每次引用转换是
独立转换操作，插值展开计数仍在一次逻辑读取内共享。
插值字符串集合按源顺序逐项展开并转换，且所有元素共享一个转换会话。这样首个插值或转换错误会
立即返回并保留原始集合索引，处理过程中最多只有一个已展开元素在途。集合值不会被拆分；标量列表
文本仍遵循配置的拆分策略。现有有效输入和公共调用方式无需迁移。
保存的 overlay 按累计聚合预算准入；单个解析文本仍有单值限制。启用累计准入时，
预算超限错误可能优先于后续解析错误。

## 结构化 Serde

`ConfigSerdeExt` 会优先选择精确配置项；如果不存在，则根据请求前缀下的子项构造
JSON 风格对象。同一前缀同时存在精确配置项和子项时返回 `KeyConflict`。空前缀表示
reader 当前可见的全部配置项。

`UnknownFieldPolicy::Reject` 会把源反序列化器收到 ignored-value 请求时暴露的字段作为
`ConfigError::UnknownProperties`，并报告排序后的根相对路径。这不等于完整 schema 校验：
flatten struct、untagged enum 和自定义反序列化器可能缓冲字段而不发出此类请求，
flatten map 也可能消费任意 key。闭合的普通 struct 可使用
`#[serde(deny_unknown_fields)]`（untagged enum 的每个 struct payload 也应使用）；Serde 返回
经过脱敏的反序列化错误，不保证是 `UnknownProperties`。该属性不能与 `flatten` 组合；要求闭合
schema 时，应使用闭合的嵌套 struct，或在反序列化前按应用 schema 校验 key。对于明确开放的
配置区域，应选择 `UnknownFieldPolicy::Ignore`。`deserialize(prefix)` 默认不插值并拒绝可观测的
未知字段；
`deserialize_with(prefix, ConfigDeserializeOptions)` 可设置 `interpolate` 和
`unknown_fields: UnknownFieldPolicy::{Reject, Ignore}`，结构化读取只保留这两个入口。
查找、插值和转换错误会保留配置上下文；只有 Serde
报告的结构不匹配才会转换为已脱敏的 `DeserializeError`。投影遵循 Serde 的 JSON
风格数据模型，不承诺覆盖 `ConfigReader::get` 为富类型提供的所有原生转换形态。
对象成员名会以字符串交给 Serde 的 key deserializer；数字 map key 需要应用显式解析。
请求 `deserialize_bytes` 或 `deserialize_byte_buf` 的目标会按配置的转换规则将选中值转换为
`String`，再取得其 UTF-8 字节。普通 `Vec<u8>` 则请求数值序列；不会隐式执行 Base64 或
十六进制解码。第三方字节包装类型也走 bytes 接口，而非 sequence 接口。
结构化读取的诊断路径对普通成员名保留点分形式；含 `.`, `[`, `]`, `"`, `\\` 或控制字符的
成员名使用 JSON 字符串转义的方括号段。例如字面成员 `a.b` 显示为 `payload["a.b"]`，
嵌套成员仍显示为 `payload.a.b`；数组位置仍显示为 `[index]`。选中的配置根路径中特殊段也
使用相同编码。这些字符串仅用于诊断，不可传给 `Config::get` 或 section 查询。普通 reader
路径与 Wire JSON Pointer 保持各自原有格式。直接比较特殊键诊断字符串的调用方应更新比较值；
配置键和查询行为不变。
结构化读取原生 `HashMap<String, String>` 时按原始键升序访问成员，使每个映射内的错误在独立实例间保持
稳定。该规则保留阶段优先级和数组顺序；多个嵌套准备错误同时存在时，不承诺选择全树路径字典序最小的错误。
插值查找作用域取决于选中的存储形态：点分后代从选中的子树查找，精确结构化属性则从调用
reader 查找。对象成员名只是数据，不会自动成为 `Config` key；根配置和显式允许的环境变量
回退规则仍然适用。详见[结构化读取说明](user_guide.zh_CN.md#反序列化结构化值)。

准备树保存借用值、集合 slice、JSON 节点和合成对象索引，不克隆完整 JSON 中间载荷。
NodeId 索引 arena 节点及对应的来源表；property index 则索引单独的借用配置项表。来源位置由
原始配置项和有类型的嵌套 key segment 组成，因此 JSON key 中的字面点号不会与点分配置路径混淆。
规范请求前缀保存在 `root_path` 中用于查找和字节偏移；结构化诊断使用从它派生的单独编码根路径。
两个对象贡献不同叶子时可以合并；重复叶子和标量/对象冲突会失败。插值覆盖使用带类型的源位置，
JSON 键中的字面点号不会与嵌套路径混淆。只展开真正的字符串叶子，每个叶子处理一次；
URL 等富类型格式化后的文本不作为插值输入，未变化文本保持借用。

执行 visitor 前，对完整选中输入准入，包括目标忽略的字段。插值读取分别检查原始输入和展开后
输入。这些检查取 reader 的转换限制，但计数器与叶子共享的 `ConversionSession` 独立。
明确的数值目标从原始运行时类型转换；`deserialize_any` 遵循自然 JSON 类别，包括宽整数的
十进制文本表示。标量列表拆分复用已准入元素，真实集合内的字符串不再拆分。
JSON 整数字面量在有符号或无符号 128 位范围内精确保留。超范围整数字面量和非有限浮点值
返回转换错误；有限浮点数沿用既有二进制浮点语义。通用 wire 反序列化器即使在依赖图中统一启用了
`serde_json/arbitrary_precision`，也会按对应的 schema 角色类型化读取 Float32/Float64。任意 JSON
领域数字仍遵循通用 Serde 事件语义；需要保留 JSON 数字语义时，应使用专用 JSON helper。
精确标量被策略判定缺失时可读取为 `None`，子树中的缺失标量字符串会被省略，集合某项缺失
仍然报错。公共返回值仍要求 `DeserializeOwned`。

## Wire 持久化

Serde 序列化会输出确定性的 V1 JSON 封装，其中包含显式 `version` 字段，配置键按
字典序排列。解码同时接受 V1 和旧版无版本顶层表示，会校验规范映射键与配置项内部
名称，并拒绝未知版本。运行时读取策略有意不进入 V1 持久化契约；旧表示中的策略数据
可以读入，但不会应用。
V1 拒绝任何显式 `read_options` 字段，包括 `null`；legacy 无版本表示仍接受并忽略该字段。
手工构造 V1 封装的生产者应省略 `read_options`。

`Config::encode_json_vec` 和 `Config::decode_json_slice` 使用默认的有限 wire 配置；
对应的 `*_with_limits` 变体接收自定义 `ConfigWireLimits`。普通 Serde
`Deserialize` 可以限制已解码值，却无法对原始输入字节或 JSON 词法 token 做准入；
完整的不可信 JSON 输入必须使用 `decode_json_slice`。

两条 wire 解码路径都会先拒绝重复对象键，再检查配置项数量与 UTF-8 键字节数，并在构造类型化
`Property` 前完成这些领域限额检查。受限 JSON 入口会先在 JSON profile 下物化唯一键 JSON 树，
因此领域检查不会阻止上游树分配。若属性值同时无效且超过领域限额，会优先报告限额错误；
限额满足时仍保留现有 Serde 属性错误。

有界表示失败使用 `ConfigWireDecodeError::Representation { path, reason }`。路径是相对于
wire 文档根的 JSON Pointer，因此 `/properties/server.port/value` 中的点属于单个 property
名称；它与点分配置路径不同。预检会识别缺失/未知封装字段和封装字段类型错误，其他深层
失败安全地归类为 `InvalidValue`，不包含输入值，也不声称保留原始源码坐标。业务错误的
可观察分类从 `Json(Data)` 变为 `Representation`；语法、预算和领域限额错误仍分别处理。

持久化序列化与 `ConfigSerdeExt` 读取业务结构体是不同能力。本次不增加业务结构体反向展平
写入配置的 serializer，现有 Wire V1 表示保持不变。

## 资源预算

crate 为不同操作使用相互独立的资源域，而不是共享一个计数器：

| 资源域 | 默认限制 | 作用边界 |
| --- | --- | --- |
| 类型转换 | `qubit-datatype` 的 `ConversionLimits::default()` | 单次普通读取，或一次完整的结构化物化 |
| 结构化输入准入 | reader 的转换操作与结构值限制 | 完整选中输入；插值前后分别检查，计数独立 |
| 插值 | 深度 64；展开 4,096 次；输出 1 MiB | 单次插值读取 |
| 配置源加载 | 输入 8 MiB；赋值 65,536 个；节点 262,144 个；parser token 1,048,576 个；parser depth 128；每作用域累计准入后代来源 256 个；语义深度 64；固定 composite 调度深度 256 条边 | Token 在单个 source 和所有外层 composite 间累计；parser depth 按每个输入检查；其他累计维度适用局部与外层作用域；固定调度保护以根深度 0 计，独立于 `SourceLimits` |
| JSON wire | 输入/输出 1 MiB；深度 64；节点 100,000 个；序列项、映射项和配置项各 4,096 个；字符串和对象键 256 KiB；数字 4 KiB；载荷 1 MiB；配置项键 256 字节 | 单次编码或解码 |

来源数限额表示此 composite 作用域内累计准入的后代来源数，不计作用域根。
进入嵌套 composite 会在父作用域计 1；该 composite 后续准入的所有后代也会在每个外层作用域计费。
例如 `outer -> inner -> leaf-a, leaf-b` 中，`outer` 至少需要 3，`inner` 至少需要 2。
计数按准入次数计算，包括空来源和后来被覆盖的配置层，不按对象去重或最终属性数计算；每次加载都会重置预算。
自定义来源仍须自行报告其工作量。此限额约束准入来源数，不约束 composite 嵌套深度。
历史 RSC-R03 [组合来源资源探测记录](composite-resource-probe-20261106T065653653Z-81411945.md)
显示在固定执行器保护加入前，提高或取消来源数限额时，1,025 条边的深链曾触发栈溢出；该结果促成独立的固定执行器保护。
当前内置 composite 调度在第 257 条活动子来源边返回 `SourceLimitExceeded` 和
`SourceLimitKind::SourceNestingDepth`，即使来源限额提高或设为 unbounded 也一样。根深度为 0，检查顺序在
source-count 准入之后，因此已有 `SourceCount` 错误仍优先。新的受监督探测覆盖 255/256/257 和 1,025 条边的链、
1,025 个子来源的宽图、debug/release、2/8 MiB 栈以及 default/raised/unbounded 策略；测试链均完成加载或返回结构化错误，
并到达销毁阶段。这些单机结果不保证所有平台的栈安全，也不覆盖自定义递归或任意来源树销毁。请扁平化深层内置组合，
并保留保守的有限来源数限额。

配置源的字节、赋值、节点、parser token 和子配置源计数是累计预算，只有全部适用的局部与聚合
作用域都接受一次扣减时才会共同提交；语义深度和 parser depth 是点限制。TOML lexer token
（包括空白、注释、换行和 EOF）与 YAML scanner token 都在 AST backend 前计费。YAML depth
从根 0 开始按 collection 嵌套；TOML depth 是保守上界，每个点分 key 段最多计两层，并叠加
array/inline container 深度；它不等同于语义 `max_nesting_depth`。原有 node、assignment 和语义深度
仍在 flatten 阶段检查。旧版序列化 `SourceLimits` 缺少 parser 字段时使用新默认值；
`SourceLimits::unbounded()` 也会关闭这两个维度。Wire 预算与 `SourceLimits`、读取转换限制互相独立。
`ConfigWireLimitsBuilder::max_input_bytes` 只修改输入字节维度并保留其他解码限制；
`json_decode` 用于整体替换解码 profile。

### TOML/YAML 解析器资源限制

完整矩阵与原始进程记录见 [2026-11-01 测量报告](https://github.com/qubit-ltd/rs-config/blob/main/doc/resource-probe-20261101T042738659Z-7be245fc.md)。
报告在 1 GiB 地址空间上限下复现 YAML 8 MiB、419 万元素宽序列的分配中止，最大 RSS 约
613 MiB；降低节点/深度预算也未阻止失败。按源码调用顺序，失败发生在
`serde_norway::from_str` 内、展平之前，但探针无法区分 scanner 与 parser 的具体分配点。
对应 TOML 样本达到约 640 MiB RSS 后才在展平阶段返回节点预算错误。这些测量均在加入解析阶段准入前完成，
并促成了上文的 parser token 与 parser depth 限制；
它们不是应用 SLO，也不构成普遍安全阈值。新检查不限制单个 scanner token 的内存分配、CPU 时间，
也不消除通过准入后的 AST 成本。调用方使用 `SourceLimits::unbounded()` 时也会关闭新限制。

[资源探针](../examples/config_source_resource_probe.rs)提供了可选的手动入口。对于 `wide`，`SIZE`
表示数组元素数；生成输入的实际字节数以输出的 `input_bytes` 为准。`load` 可能因资源限额拒绝输入
而以非零状态退出，`generate-only` 只生成输入，不加载配置：

```bash
cargo run --locked --example config_source_resource_probe --features formats -- toml wide 524288 16 8 load
cargo run --locked --example config_source_resource_probe --features formats -- yaml wide 524288 16 8 generate-only
```

后续验收记录见
[`resource-probe-20261101T124404Z-rsc101.md`](https://github.com/qubit-ltd/rs-config/blob/main/doc/resource-probe-20261101T124404Z-rsc101.md)。
探针报告实际错误维度和进程 RSS；通过结果仅适用于记录的工具链、机器、输入与限额。

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

### 迁移说明

升级下游 crate 前，请先阅读[从 qubit-config 0.14 迁移](user_guide.zh_CN.md#从-qubit-config-014-迁移)。当前依赖图可能同时包含 0.14 与 0.15；两个版本公开的 `Config`、`ConfigReader` 类型不同，不能跨版本互传。具体本地依赖约束以用户指南中的核对结果为准。

升级时应检查 TOML/YAML 中作为 mapping 成员名使用的空键或无效点分键；这些输入现在会在
递归前作为来源解析错误拒绝，而合法空对象仍可用。环境变量名称归一化后若发生冲突，错误仍是
`KeyConflict`，但现在会稳定报告规范化键最小组及其中名称最小的两项。此说明只描述本库合同，
不声称已在所有下游应用中完成验证。

此次诊断增强为 `ConfigError::SourceParseError` 增加公开的 `location` 字段，改变了公开错误变体布局；
直接构造和穷举字段解构都需要源码迁移。升级到未来的 0.16 时，直接构造若无已知坐标，应填写
`location: None`；穷举解构应增加 `..` 或绑定 `location`。使用 `{ .. }` 的匹配无需修改。
该变化必须随下一个不兼容 minor 版本发布，不得描述为与 0.15 的 patch 版本完全兼容。

rs-http 配置适配器将未知键校验和 `usize` 读取委托给 `ConfigReader`。超出平台 `usize` 范围的整数现在
会以 `PlatformIntegerOutOfRange`（`Conversion`）返回，并映射为 `HttpConfigErrorKind::TypeError`；这有意
替换原来 32 位 helper 自行生成的 `DeserializeError` 分类。成功值和 helper 签名不变。

受支持的使用方式请参阅[用户手册](user_guide.zh_CN.md)、
[API 文档](https://docs.rs/qubit-config)和[项目 README](../README.zh_CN.md)。
