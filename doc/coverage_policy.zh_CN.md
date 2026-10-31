# qubit-config 覆盖率策略

[English](coverage_policy.md) | 简体中文

本文说明 `qubit-config` 的 package 级逐文件覆盖率门槛。项目使用共享覆盖率生成器完成
插桩和报告生成，再依据本项目策略检查 `src/` 下的文件。

## 强制规则

使用以下命令运行完整的本地覆盖率流程：

```text
./scripts/check-project-coverage.sh
```

脚本先运行 `.infra/bin/coverage.sh`，与 CI 一样启用增量编译，在仓库根目录生成
报告 `coverage.json`，再调用：

```text
python3 -B scripts/check-coverage-files.py coverage.json --config .infra/coverage/coverage.json
```

检查器还会默认读取 `scripts/coverage-metric-overrides.json`。可用
`--metric-overrides PATH` 指定其他项目 sidecar。单元测试和 pipeline fixture
通过检查器默认路径提供各自隔离的 sidecar。

CI 的 `coverage-files` job 会下载 `rust-ci` job 生成的 `coverage-reports` artifact，并使用相同的
`.infra/coverage/coverage.json` 策略检查 `artifacts/coverage/coverage.json`。共享覆盖率汇总和 badge
仍可用于查看整体情况，但不能替代逐文件门槛。

策略作用域为 package `qubit-config`，源码目录为 `src/`。每个未豁免源码文件都必须满足：

- functions 不低于 95%；
- lines 高于 90%；
- regions 高于 85%。

每个被选中的生产文件都必须出现在报告中，且 functions、lines、regions 三项计数均有效，
包括配置了指标下限的文件。没有 override 时，functions 至少为 95%，lines 必须高于 90%，
regions 必须高于 85%。sidecar 下限只应用于明确列出的指标，并使用包含边界的 `>=`；
其他指标继续使用正常门槛。只有指标低于正常门槛时才使用 override，因此已满足正常规则的
指标不计为例外。sidecar、版本、必需文件条目、理由、证据或有效 summary 缺失时检查器失败，
不会回退为整文件豁免。`checked_files` 包含所有可执行生产文件；`exempt_files` 表示至少有
一个指标正在使用较低下限的文件数。
在普通阈值的精确等号处，lines 和 regions 仍须使用 override，因为其普通规则是严格大于（`>`）；
functions 正好为 95% 时已满足普通规则（`>=`），不计为活跃例外。

当前 sidecar 来自 2026-10-31 UTC 生成的新报告，工具为 `cargo-llvm-cov 0.8.6`、Rust 1.94.0、
LLVM 21.1.8，命令为 `CARGO_INCREMENTAL=1 ./.infra/bin/coverage.sh`。报告基于 HEAD
`8ea2d29fff44d06b3d1118dcac5225336929d228` 的工作树，并应用了原工作区已有的 wire 修改
以运行完整测试套件；仍需 override 的两个文件 SHA-256 已记录在 sidecar 中。三个测量文件的计数如下：

| 文件 | Functions | Lines | Regions | 生效下限 |
| --- | ---: | ---: | ---: | --- |
| `src/error/config_error.rs` | 100.00% (20/20) | 91.10% (215/236) | 88.09% (244/277) | 无 |
| `src/source/toml_config_source.rs` | 77.50% (31/40) | 92.53% (223/241) | 83.62% (383/458) | functions >= 77.50%；regions >= 83.62% |
| `src/source/yaml_config_source.rs` | 84.09% (37/44) | 93.84% (259/276) | 88.74% (465/524) | functions >= 84.09% |

sidecar 为每个有 override 的文件记录三项指标的完整精度百分比及 covered/count。生效下限取实际百分比向下
保留两位小数；达到正常门槛的指标不设置 override。此前移除的六个旧例外继续保持移除：
`src/config/access.rs`、`src/key/config_key.rs`、`src/key/config_path.rs`、
`src/property/property.rs`、`src/property/property_mut.rs` 和 `src/reader/config_section.rs`。
共享 `.infra/coverage/coverage.json` 遵循 `.infra/bin/coverage.sh` 使用的严格 schema。项目 checker
在 `.infra/coverage/non-executable-files.json` 中显式记录仅含声明的文件和仅供测试使用的模块，
不会仅因报告缺少某个文件就静默忽略它。

## 维护策略

不得将业务分支加入例外。新增下限需要最新覆盖率报告、精确的计数归属缺口或不可达防御分支，
以及覆盖对应可观察行为的 focused test。`minimums` 只列出仍低于正常门槛的指标，不得使用整文件
放行。修改机器可读 sidecar 时，须同步更新中英文文档。Rust/LLVM 或 `cargo-llvm-cov` 修复计数
归属问题后，应移除对应下限。

已有报告可单独执行检查：

```text
python3 -B scripts/check-coverage-files.py coverage.json --config .infra/coverage/coverage.json
```

指定其他证据文件时，增加 `--metric-overrides PATH`。
