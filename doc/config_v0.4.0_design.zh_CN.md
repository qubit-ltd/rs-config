# `rust-config` `v0.4.0` 设计文档

## 文档信息

- 文档名称：`rust-config v0.4.0 设计文档`
- 文档版本：`v1.0`
- 创建日期：`2026-04-09`
- 当前版本：`0.3.0`
- 目标版本：`0.4.0`
- 前置假设：`qubit-value v0.3.0` 已支持 `usize/isize`、`Duration`、`Url`、`HashMap<String, String>` 与 `Json`

## 1. 背景

`qubit-config` 当前已经能完成“配置来源加载 + 扁平 key/value 存储 + 泛型读取”，但它更接近基础配置容器，还不具备成熟的“结构化配置映射层”能力。

这在下面几类场景中成为瓶颈：

1. 按前缀提取模块配置，例如 `http.*`、`db.*`。
2. 把 dotted keys 映射成业务 struct。
3. 区分“缺失”“空值”“null”和“显式 None”。
4. 让 TOML/YAML 的原始标量类型尽量不丢失。
5. 让环境变量和 properties 文件也能可靠承载 map、enum、duration、url 等结构。

## 2. 目标

`v0.4.0` 的目标是把 `qubit-config` 从“类型安全配置容器”提升为“结构化配置映射层”，让上层 crate 可以稳定地从多个来源读取配置，并恢复为明确的领域配置对象。

## 3. `v0.4.0` 必须补齐的能力

### 3.1 按前缀遍历与子树提取

新增能力：

1. `iter()` 或等价遍历接口。
2. `iter_prefix(prefix)`。
3. `contains_prefix(prefix)`。
4. `subconfig(prefix, strip_prefix)`。

目标：

1. 允许上层以模块维度读取配置。
2. 允许从 `http.proxy.host` 这类扁平 key 中恢复出局部配置视图。
3. 让 `Config -> 结构化对象` 的实现不必先拿全量 key 再手工过滤。

### 3.2 Optional 与 null 语义

新增能力：

1. `get_optional<T>()`
2. `get_list_optional<T>()`
3. `is_null(name)` 或等价语义

要求：

1. “配置不存在”和“配置存在但值为 null/None”必须可区分。
2. TOML/YAML 的 null 不能再被简单退化为空字符串。
3. 结构化对象中的 `Option<T>` 需要能稳定映射。

### 3.3 结构化配置映射

新增能力：

1. `get_struct<T>(prefix)` 或 `deserialize<T>(prefix)`。
2. 支持从 dotted keys 恢复嵌套 struct。
3. 支持 `Option<T>`、`Vec<T>`、`HashMap<String, String>`、enum、`Duration`、`Url`。

目标：

1. 上层 crate 不再需要每个字段手工 `get()`。
2. `qubit-config` 成为统一的配置映射入口，而不是只提供原语。

## 4. 配置来源加载的增强需求

### 4.1 TOML / YAML 类型保真

`v0.4.0` 需要尽量保留来源中的原始标量类型，而不是把所有标量统一转成字符串。

要求：

1. 整数保持整数。
2. 浮点保持浮点。
3. 布尔保持布尔。
4. null 保持 null。
5. 标准数组保持列表。
6. map 结构在内部可恢复为子树视图。

说明：

`Config` 仍然可以继续以扁平 key 作为外部访问接口，但 source loader 不能再无条件把结构信息提前抹平。

### 4.2 环境变量与 properties 的结构化输入

对纯文本来源，`v0.4.0` 需要定义稳定的解析策略。

建议规则：

1. `Duration` 允许文本格式，例如 `10s`、`120s`、`500ms`。
2. `Url` 允许标准 URL 字符串。
3. enum 默认走字符串解析。
4. `HashMap<String, String>` 支持两种输入：
   - 通过前缀子键输入，例如 `HTTP_DEFAULT_HEADERS_AUTHORIZATION=xxx`
   - 通过 JSON 文本输入，例如 `HTTP_DEFAULT_HEADERS={"Authorization":"xxx"}`
5. 复杂对象通过 JSON 文本输入。

这条规则尤其适合 env / `.properties` 这类天然只有字符串的配置来源。

## 5. 错误模型增强

`v0.4.0` 需要让错误具备更强的定位能力。

建议增强：

1. `TypeMismatch` 携带 key/path。
2. `ConversionError` 携带 key/path 与目标类型。
3. 结构化反序列化错误携带字段路径。
4. source 解析错误区分“来源格式错误”和“类型映射错误”。

目标：

1. 当 `http.logging.body_size_limit` 配置错误时，错误信息里必须直接出现该路径。
2. 不再出现丢失 key 名的空错误上下文。

## 6. 建议新增 API 轮廓

建议新增以下一组 API：

```rust
impl Config {
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Property)>;
    pub fn iter_prefix<'a>(&'a self, prefix: &'a str)
        -> impl Iterator<Item = (&'a str, &'a Property)>;
    pub fn contains_prefix(&self, prefix: &str) -> bool;
    pub fn subconfig(&self, prefix: &str, strip_prefix: bool) -> ConfigResult<Config>;

    pub fn get_optional<T>(&self, name: &str) -> ConfigResult<Option<T>>;
    pub fn get_list_optional<T>(&self, name: &str) -> ConfigResult<Option<Vec<T>>>;

    pub fn deserialize<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: serde::de::DeserializeOwned;
}
```

命名可以调整，但上述语义必须具备。

## 7. 与 `qubit-value v0.3.0` 的协作方式

### 7.1 原生读取的类型

在 `qubit-value v0.3.0` 前提下，`qubit-config v0.4.0` 需要直接支持读取：

1. `usize`
2. `isize`
3. `Duration`
4. `Url`
5. `HashMap<String, String>`
6. `serde_json::Value`

### 7.2 enum 与复杂结构

建议策略：

1. 简单 enum 默认先尝试字符串解析。
2. 如果目标类型支持 serde，则允许从 JSON 路径恢复。
3. 配置源为 JSON 文本时，可先解析为 `serde_json::Value`，再映射到目标类型。

## 8. 对上层 crate 的直接价值

### 8.1 对 `qubit-http`

`qubit-http` 可以在 `v0.2.0` 里直接获得：

1. `Config -> HttpClientOptions`
2. `Config -> TimeoutOptions`
3. `Config -> ProxyOptions`
4. `Config -> HttpLoggingOptions`

并且支持：

1. `http.default_headers.*` 形式的子键输入
2. JSON 文本形式的 map 输入
3. `Duration`、`Url`、enum 的结构化解析

### 8.2 对其他模块

同样适用于数据库客户端、缓存客户端、消息队列客户端等任何需要“按模块前缀恢复结构化配置”的场景。

## 9. 非目标

以下内容不属于 `v0.4.0` 必做范围：

1. 把 `Config` 改造成完整对象树并废弃 dotted keys
2. 强绑定某个业务 schema 框架
3. 直接内建 HTTP、数据库等领域对象

## 10. 验收标准

1. 支持 prefix 遍历、prefix 判断、子配置提取。
2. 支持 `Option<T>` 与 null 语义。
3. 支持从子配置反序列化为结构化对象。
4. TOML/YAML source 不再把所有标量无差别降级为字符串。
5. env / properties 支持通过字符串规则或 JSON 文本承载复杂配置。
6. 错误信息包含失败 key/path。
