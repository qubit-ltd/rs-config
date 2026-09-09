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

以下证据于 2026-09-10 使用 `cargo-llvm-cov 0.8.6` 生成，来源为执行上述命令后的
`target/llvm-cov/coverage.json`。该 JSON 没有可用的源码 branch 计数
（`branches.count` 为零），因此门槛使用 regions 作为对分支敏感的证据。无需重新
运行测试即可用下列命令复现带标注的计数视图：

```text
cargo llvm-cov report --text --show-missing-lines
```

## 当前插桩例外

下表是 `.rs-ci-coverage.json` 当前八个路径的精确有序副本。百分比和计数均来自上述
证据报告。

| 文件 | Functions | Lines | Regions | 计数形态与行为证据 |
| --- | ---: | ---: | ---: | --- |
| `src/config/access.rs` | 94.12% (16/17) | 95.83% (69/72) | 92.97% (119/128) | 转发访问器存在内联归因缺口；section 测试覆盖存在、缺失和相对路径行为。 |
| `src/error/config_error.rs` | 100.00% (16/16) | 87.91% (160/182) | 86.07% (173/201) | 错误分类和来源适配包含泛型与防御性分支；错误测试覆盖类型、路径、missing 事实和 source 链。 |
| `src/key/config_key.rs` | 85.71% (6/7) | 88.89% (24/27) | 89.19% (33/37) | 键访问器存在内联归因缺口；键测试覆盖 AsRef、Unicode、空白、非法键和 Serde 校验。 |
| `src/key/config_path.rs` | 85.71% (12/14) | 89.66% (52/58) | 88.89% (72/81) | 路径访问器存在内联归因缺口；路径测试覆盖根路径、相对路径、校验和 Serde。 |
| `src/property/property.rs` | 75.00% (15/20) | 85.85% (91/106) | 82.20% (97/118) | 五个总被内联的访问器/修改器（`value_mut`、`description`、`set_description`、`data_type` 和 `len`）即使被直接测试调用，函数体仍显示零计数。`property_tests` 覆盖标量与集合修改、元数据、final 标记、unset 状态、类型与长度、克隆和 wire 往返；`config_property_mut_tests` 覆盖通过配置 facade 执行的修改。 |
| `src/reader/config_section.rs` | 90.91% (30/33) | 92.67% (139/150) | 91.34% (232/254) | 薄转发方法和泛型 AsRef 调用存在归因缺口；section 测试覆盖嵌套路径、可见性、迭代、策略和空 section。 |
| `src/source/toml_config_source.rs` | 78.95% (30/38) | 92.48% (209/226) | 83.53% (350/419) | feature 控制的构建会在不同测试二进制中复制泛型 builder/转换函数和迭代器/错误闭包。标量转字符串函数中 integer、float、boolean 和嵌套值的回退分支属于防御逻辑：同构数组在调用该转换器前已完成分派，嵌套数组/表也已提前拒绝。`toml_config_source_tests` 覆盖所有可接受的标量/数组类型、混合与嵌套拒绝、解析/I/O 错误、冲突、限制、final 值和事务语义。 |
| `src/source/yaml_config_source.rs` | 81.25% (39/48) | 87.33% (317/363) | 83.31% (549/659) | feature 控制的泛型序列转换器、扫描闭包和错误适配器存在重复的零归属实例。嵌套序列的 `unreachable!` 分支以及标量转字符串函数中的嵌套值分支属于防御逻辑，因为前置扫描会先拒绝 mapping、sequence 和 tagged 项；无法通过公共加载路径构造没有公开位置的解析器错误。`yaml_config_source_tests` 覆盖可接受的标量/序列/tagged 形式、alias 拒绝和引号/块标量例外、非字符串键、混合与嵌套拒绝、冲突、限制、final 值和事务语义。 |


本次重构删除了旧 scalar sequence 实现及其豁免；`config_wire_limits.rs` 和 `property_mut.rs` 在新报告中已满足全部门槛，故也移除豁免。新 structured_read 模块不设豁免。

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
