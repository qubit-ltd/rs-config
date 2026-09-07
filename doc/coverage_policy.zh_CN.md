# qubit-config 覆盖率策略

[English](coverage_policy.md) | 简体中文

本文记录 `.rs-ci-coverage.json` 中各文件暂时不参与逐文件门槛检查的原因。只有
LLVM 插桩或计数归属限制，或者因前置不变量检查而无法到达的防御分支，才能成为
例外；例外不能用来放任生产行为缺少测试。

## 强制规则

权威覆盖率命令为：

```text
COVERAGE_ENFORCE_THRESHOLDS=1 ./coverage.sh json
```

该命令启用所有 Cargo feature，并把机器可读报告写入
`target/llvm-cov/coverage.json`。共享覆盖率生成器成功后，项目的 `coverage.sh`
wrapper 会立即运行 `scripts/check-coverage-files.py`。检查器读取这份新生成的报告和
`.rs-ci-coverage.json`，只排除已配置的插桩例外，然后检查报告中 `src/` 下的每个
剩余生产文件：

- functions 不低于 95%；
- lines 必须高于 90%；
- regions 必须高于 85%。

`coverage.sh` 输出的汇总结果不能替代逐文件门槛。否则，crate 整体的高覆盖率可能
掩盖单个低覆盖率文件。
JSON 缺失或无效时 wrapper 会失败，绝不会把检查当作已跳过。

项目专用 hook 的执行时间早于共享 CI runner 的覆盖率步骤，因此不负责该 gate。
根目录的 `ci-check.sh` wrapper 会保留新生成的 JSON，待共享 runner 完成后调用逐文件
检查器，之后才按请求的策略清理构建产物。该顺序保证 clean checkout 与权威覆盖率
命令执行同一个 gate。

以下证据于 2026-09-07 使用 `cargo-llvm-cov 0.8.6` 生成，来源为执行上述命令后的
`target/llvm-cov/coverage.json`。该 JSON 没有可用的源码 branch 计数
（`branches.count` 为零），因此门槛使用 regions 作为对分支敏感的证据。无需重新
运行测试即可用下列命令复现带标注的计数视图：

```text
cargo llvm-cov report --text --show-missing-lines
```

## 当前插桩例外

下表是 `.rs-ci-coverage.json` 当前十一个路径的精确有序副本。百分比和计数均来自上述
证据报告。

| 文件 | Functions | Lines | Regions | 计数形态与行为证据 |
| --- | ---: | ---: | ---: | --- |
| `src/config/access.rs` | 90.91% (20/22) | 92.68% (76/82) | 89.47% (119/133) | 两条未计数函数记录来自总被内联的 `Config::section` 和 `Config::section_if_present` 转发访问器。`inherent_section_accessors_preserve_presence_semantics` 通过经过 black box 的函数指针调用两者，并断言 section 存在与缺失时的行为，但它们的 out-of-line 定义仍为零计数。 |
| `src/conversion/internal/config_scalar_seq_access.rs` | 80.00% (4/5) | 84.38% (27/32) | 83.93% (47/56) | 泛型 `SeqAccess::next_element_seed` 的单态化及其错误闭包会在不同集成测试二进制中产生零计数副本。`test_scalar_list_error_preserves_original_index`、`test_scalar_list_visitor_can_stop_before_tail_limit`、`test_scalar_string_sequence_charges_source_and_items_once` 和 `test_scalar_string_sequence_preserves_admitted_item_errors` 分别覆盖成功、提前停止、带下标错误和受预算约束的序列读取。 |
| `src/config_wire_limits.rs` | 90.48% (19/21) | 94.50% (103/109) | 92.31% (96/104) | 两条未计数记录来自总被内联的标量限制 getter：`max_properties` 和 `max_property_key_bytes`。`config_wire_scalar_limit_getters_are_callable_as_functions` 通过经过 black box 的函数指针调用两者并校验非默认值；bounded wire 测试还分别证明这两项限制会拒绝超限输入。 |
| `src/error/config_error.rs` | 86.67% (13/15) | 83.62% (148/177) | 80.00% (156/195) | 小型 `#[inline]`/`#[inline(always)]` 上下文访问器及 match 分支在不同测试二进制间的归属不一致。`test_config_error_kind_covers_every_public_variant`、`test_source_errors_expose_source_id`、`config_error_optional_context_accessors_are_callable_as_functions`、候选路径测试和配置源限制测试会直接调用公共分类及上下文访问器，其中也包含经过 black box 的函数指针调用。 |
| `src/key/config_key.rs` | 71.43% (5/7) | 77.78% (21/27) | 75.68% (28/37) | 未计数函数记录来自总被内联的文本和格式化访问器，而不是配置键校验。`key_and_path_wrapper_traits_preserve_the_validated_text` 直接调用 `ConfigKey::as_str`、`AsRef<str>`、Serde 反序列化和 `Display`；配置键边界测试覆盖空值、分隔符、Unicode 和空白字符。 |
| `src/key/config_path.rs` | 92.86% (13/14) | 94.83% (55/58) | 95.06% (77/81) | 剩余零计数记录映射到总被内联的 `ConfigPath::as_str` 函数体。`key_and_path_wrapper_traits_preserve_the_validated_text` 通过经过 black box 的函数指针调用它，并验证 `AsRef`、`Display` 和 Serde；路径测试覆盖根路径和所有校验违规类型。 |
| `src/property/property.rs` | 75.00% (15/20) | 85.85% (91/106) | 82.20% (97/118) | 五个总被内联的访问器/修改器（`value_mut`、`description`、`set_description`、`data_type` 和 `len`）即使被直接测试调用，函数体仍显示零计数。`property_tests` 覆盖标量与集合修改、元数据、final 标记、unset 状态、类型与长度、克隆和 wire 往返；`config_property_mut_tests` 覆盖通过配置 facade 执行的修改。 |
| `src/property/property_mut.rs` | 90.91% (10/11) | 94.34% (50/53) | 94.44% (68/72) | 剩余未计数记录来自总被内联的 `ConfigPropertyMut::as_property` 访问器。`test_property_mut_guard_allows_mutation_before_final` 会直接调用它并断言受保护配置项名称；focused guard 测试还覆盖成功修改、final 状态拒绝和错误路径保留。 |
| `src/reader/config_section.rs` | 89.47% (34/38) | 91.88% (147/160) | 91.12% (236/259) | 固有方法到 trait 的薄转发、总被内联的访问器、迭代器闭包和泛型 `ConfigName` 调用点会产生重复或零归属实例。`config_section_tests` 覆盖根与嵌套路径、严格相对解析、可见性、迭代边界、继承策略、空 section，并通过经过 black box 的函数指针调用 `path` 和 `contains_section`。 |
| `src/source/toml_config_source.rs` | 78.95% (30/38) | 92.48% (209/226) | 83.53% (350/419) | feature 控制的构建会在不同测试二进制中复制泛型 builder/转换函数和迭代器/错误闭包。标量转字符串函数中 integer、float、boolean 和嵌套值的回退分支属于防御逻辑：同构数组在调用该转换器前已完成分派，嵌套数组/表也已提前拒绝。`toml_config_source_tests` 覆盖所有可接受的标量/数组类型、混合与嵌套拒绝、解析/I/O 错误、冲突、限制、final 值和事务语义。 |
| `src/source/yaml_config_source.rs` | 81.25% (39/48) | 87.33% (317/363) | 83.31% (549/659) | feature 控制的泛型序列转换器、扫描闭包和错误适配器存在重复的零归属实例。嵌套序列的 `unreachable!` 分支以及标量转字符串函数中的嵌套值分支属于防御逻辑，因为前置扫描会先拒绝 mapping、sequence 和 tagged 项；无法通过公共加载路径构造没有公开位置的解析器错误。`yaml_config_source_tests` 覆盖可接受的标量/序列/tagged 形式、alias 拒绝和引号/块标量例外、非字符串键、混合与嵌套拒绝、冲突、限制、final 值和事务语义。 |

## 维护例外列表

禁止把业务分支加入例外列表。每个新增例外都必须提供最新 JSON 报告、精确的未计数
函数或防御分支，以及证明对应可观察行为的 focused test。英文与简体中文文档必须
同步修改。

Rust/LLVM 或 `cargo-llvm-cov` 更新修复计数归属后，应先删除对应例外，而不是降低或
改变门槛。每次修改策略后都要运行权威覆盖率 wrapper 和 package 清单：

```text
COVERAGE_ENFORCE_THRESHOLDS=1 ./coverage.sh json
cargo package --list --allow-dirty
```
