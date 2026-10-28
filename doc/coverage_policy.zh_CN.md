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

CI 的 `coverage-files` job 会下载 `rust-ci` job 生成的 `coverage-reports` artifact，并使用相同的
`.infra/coverage/coverage.json` 策略检查 `artifacts/coverage/coverage.json`。共享覆盖率汇总和 badge
仍可用于查看整体情况，但不能替代逐文件门槛。

策略作用域为 package `qubit-config`，源码目录为 `src/`。每个未豁免源码文件都必须满足：

- functions 不低于 95%；
- lines 高于 90%；
- regions 高于 85%。

报告或策略缺失/无效、报告中缺少预期源码文件，或未豁免文件低于门槛时，检查器都会失败。
原 checkout 当前可用的最近一份变更前报告由 `cargo-llvm-cov 0.8.6` 生成，文件时间为
2026-10-06。逐文件计数只支持保留以下例外：

| 文件 | Functions | Lines | Regions | 证据 |
| --- | ---: | ---: | ---: | --- |
| `src/error/config_error.rs` | 100.00% (16/16) | 87.91% (160/182) | 86.07% (173/201) | 错误分类和 source adapter 仍包含泛型及防御分支；错误测试覆盖类型、路径、missing facts 和 source 链。该文件自报告生成后未修改。 |
| `src/source/toml_config_source.rs` | 78.95% (30/38) | 92.48% (209/226) | 83.53% (350/419) | feature-gated 泛型转换和防御性 scalar fallback 路径导致函数/regions 未达门槛；`toml_config_source_tests` 覆盖接受值、混合/嵌套拒绝、解析/I/O 错误、冲突、limits、final 值和事务性。这些是历史计数。 |
| `src/source/yaml_config_source.rs` | 81.25% (39/48) | 87.33% (317/363) | 83.31% (549/659) | 报告早于 RSC-002，计数反映 feature-gated 泛型 sequence 转换和防御性嵌套值分支；RSC-002 移除了旧 scanner，并将 token guard 放入单独文件。这些是历史计数。 |

同一报告中另外六个旧例外已达到三个门槛并移除：`src/config/access.rs`、
`src/key/config_key.rs`、`src/key/config_path.rs`、`src/property/property.rs`、
`src/property/property_mut.rs` 和 `src/reader/config_section.rs`；历史报告不能证明它们在新构建中的覆盖情况。
部分访问方法使用 `#[inline]` 而非强制内联，以便 LLVM 归属跨测试目标的调用。本轮没有新增阈值例外。
共享 `.infra/coverage/coverage.json` 遵循 `.infra/bin/coverage.sh` 使用的严格 schema。项目 checker
在 `.infra/coverage/non-executable-files.json` 中显式记录仅含声明的文件和仅供测试使用的模块，
不会仅因报告缺少某个文件就静默忽略它。

## 维护策略

不得将业务分支加入例外。新增例外需要最新覆盖率报告、精确的计数归属缺口或不可达防御分支，以及
覆盖对应可观察行为的 focused test。修改机器可读策略时，须同步更新中英文文档。Rust/LLVM 或
`cargo-llvm-cov` 修复计数归属问题后，应移除对应例外。

已有报告可单独执行检查：

```text
python3 -B scripts/check-coverage-files.py coverage.json --config .infra/coverage/coverage.json
```
