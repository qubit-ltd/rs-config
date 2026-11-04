# 插值深度资源探测记录

## 结论

**RSC-20261104-R01 已在本次矩阵内确认。** 原递归插值实现对显式配置的高深度链和长循环会耗尽常规大小的线程栈。复现发生在独立子进程的指定栈线程中，输出阶段为 `read`，进程以 `SIGABRT` 退出，stderr 明确报告 Rust stack overflow。未发现超时、OOM、构造失败或其他异常类别。此结论限于以下工具链、平台和矩阵，不外推到任意输入或任意平台。

debug 中 R=1024、2 MiB 栈的成功链（L=1024）和超限链（L=1023）均 3/3 溢栈；R=1024、2 MiB 的长循环也 3/3 溢栈。debug 下 R=4096、8192 在 2 MiB 和 8 MiB 栈都 3/3 溢栈。release 中 R=8192、2 MiB 栈的成功链（L=8192）和超限链（L=8191）均 3/3 溢栈。所有这些输入均在本次显式展开预算 16,384 次以内、默认输出字节预算以内。栈溢出不是深度错误、循环错误或其他资源限制被误判。

## 环境与隔离

- OS：Linux `6.8.0-147-generic`，Ubuntu 内核；目标/机器架构：`x86_64-unknown-linux-gnu` / `x86_64`。
- rustc：`1.94.0 (4a4ef493e 2026-03-02)`；Cargo：`1.94.0 (85eff7c80 2026-01-15)`；Python：`3.12.3`。
- 每次探测一个全新子进程；Config 构造、插值读取和销毁都在 `std::thread::Builder::stack_size` 指定的线程中运行。单点 10 秒，超时后 kill 并 `communicate()` 回收。
- 每个子进程成功设置 `RLIMIT_AS=512 MiB` 和 `RLIMIT_CORE=0`。180 次运行均无驱动设置错误；探测输出没有环境变量或完整配置内容。发生 `SIGABRT` 的进程没有留下 core 文件。
- 栈大小为 2 MiB、8 MiB；每个唯一参数组合每种 profile 重复 3 次。默认边界使用完整默认策略；提高深度链和循环使用展开限额 16,384、默认输出限额。
- 为适配嵌套 worktree，Cargo 构建通过只在目标 worktree 内临时修订 `Cargo.toml` 的路径依赖，读取原 checkout 的 `rs-redact` 和 `rs-value`。它们当时均在 `dev-starfish` 分支：`rs-redact` HEAD `a81edd114d2e5b5aaf470931cd8de8850484276e`，存在未跟踪文档 `doc/2026-11-04-shared-logic-refactoring-design.zh_CN.md` 与 `doc/2026-11-04-shared-logic-refactoring-plan.zh_CN.md`；`rs-value` HEAD `2346145e1ce3f11e295d8bfc776fa18c315fe8b6`，存在未跟踪文档 `doc/design-20261104T152337886Z-1fc50431.md` 与 `doc/implementation-plan-20261104T152337886Z-1fc50431.md`。构建仅读取其 manifest/source；这些既存文档及仓库状态均未修改、移动或纳入本 worktree。构建完成后目标 worktree `Cargo.toml` 已恢复并通过哈希核对。
- 探测输出保存在 `target/interpolation-probe/debug` 与 `target/interpolation-probe/release`，每次运行各有 stdout/stderr，并有 `summary.csv`、`summary.json`、`counts.json` 和工具链记录。以下表格保留所有参数组合和每组 3 次的实际分类，target 清理后仍可审查矩阵结果。

## 完整矩阵

每行代表该参数点的 3 次重复。`success`、`depth-exceeded`、`cycle` 表示探测 example 对返回值／错误结构的断言通过且进程正常退出；`stack-overflow` 表示读取阶段 `SIGABRT`，stderr 包含 `thread 'interpolation-probe' ... has overflowed its stack` 与 `fatal runtime error: stack overflow, aborting`。所有行输出阶段为 `drop` 并正常退出，或（溢栈行）在 `read` 阶段退出。

| Profile | 类别 | Mode | 引用数 R | 深度限额 L | 栈 | 3 次结果 | 阶段 / 信号 |
| --- | --- | --- | ---: | ---: | ---: | --- | --- |
| debug | 默认边界 | chain | 0 | 64 | 2 MiB | success ×3 | drop / exit 0 |
| debug | 默认边界 | chain | 0 | 64 | 8 MiB | success ×3 | drop / exit 0 |
| debug | 默认边界 | chain | 1 | 64 | 2 MiB | success ×3 | drop / exit 0 |
| debug | 默认边界 | chain | 1 | 64 | 8 MiB | success ×3 | drop / exit 0 |
| debug | 默认边界 | chain | 63 | 64 | 2 MiB | success ×3 | drop / exit 0 |
| debug | 默认边界 | chain | 63 | 64 | 8 MiB | success ×3 | drop / exit 0 |
| debug | 默认边界 | chain | 64 | 64 | 2 MiB | success ×3 | drop / exit 0 |
| debug | 默认边界 | chain | 64 | 64 | 8 MiB | success ×3 | drop / exit 0 |
| debug | 默认边界 | chain | 65 | 64 | 2 MiB | depth-exceeded ×3 | drop / exit 0 |
| debug | 默认边界 | chain | 65 | 64 | 8 MiB | depth-exceeded ×3 | drop / exit 0 |
| debug | 提高深度 | chain | 256 | 256 | 2 MiB | success ×3 | drop / exit 0 |
| debug | 提高深度 | chain | 256 | 256 | 8 MiB | success ×3 | drop / exit 0 |
| debug | 提高深度 | chain | 256 | 255 | 2 MiB | depth-exceeded ×3 | drop / exit 0 |
| debug | 提高深度 | chain | 256 | 255 | 8 MiB | depth-exceeded ×3 | drop / exit 0 |
| debug | 提高深度 | chain | 1024 | 1024 | 2 MiB | stack-overflow ×3 | read / SIGABRT |
| debug | 提高深度 | chain | 1024 | 1024 | 8 MiB | success ×3 | drop / exit 0 |
| debug | 提高深度 | chain | 1024 | 1023 | 2 MiB | stack-overflow ×3 | read / SIGABRT |
| debug | 提高深度 | chain | 1024 | 1023 | 8 MiB | depth-exceeded ×3 | drop / exit 0 |
| debug | 提高深度 | chain | 4096 | 4096 | 2 MiB | stack-overflow ×3 | read / SIGABRT |
| debug | 提高深度 | chain | 4096 | 4096 | 8 MiB | stack-overflow ×3 | read / SIGABRT |
| debug | 提高深度 | chain | 4096 | 4095 | 2 MiB | stack-overflow ×3 | read / SIGABRT |
| debug | 提高深度 | chain | 4096 | 4095 | 8 MiB | stack-overflow ×3 | read / SIGABRT |
| debug | 提高深度 | chain | 8192 | 8192 | 2 MiB | stack-overflow ×3 | read / SIGABRT |
| debug | 提高深度 | chain | 8192 | 8192 | 8 MiB | stack-overflow ×3 | read / SIGABRT |
| debug | 提高深度 | chain | 8192 | 8191 | 2 MiB | stack-overflow ×3 | read / SIGABRT |
| debug | 提高深度 | chain | 8192 | 8191 | 8 MiB | stack-overflow ×3 | read / SIGABRT |
| debug | 长循环 | cycle | 256 | 257 | 2 MiB | cycle ×3 | drop / exit 0 |
| debug | 长循环 | cycle | 256 | 257 | 8 MiB | cycle ×3 | drop / exit 0 |
| debug | 长循环 | cycle | 1024 | 1025 | 2 MiB | stack-overflow ×3 | read / SIGABRT |
| debug | 长循环 | cycle | 1024 | 1025 | 8 MiB | cycle ×3 | drop / exit 0 |
| release | 默认边界 | chain | 0 | 64 | 2 MiB | success ×3 | drop / exit 0 |
| release | 默认边界 | chain | 0 | 64 | 8 MiB | success ×3 | drop / exit 0 |
| release | 默认边界 | chain | 1 | 64 | 2 MiB | success ×3 | drop / exit 0 |
| release | 默认边界 | chain | 1 | 64 | 8 MiB | success ×3 | drop / exit 0 |
| release | 默认边界 | chain | 63 | 64 | 2 MiB | success ×3 | drop / exit 0 |
| release | 默认边界 | chain | 63 | 64 | 8 MiB | success ×3 | drop / exit 0 |
| release | 默认边界 | chain | 64 | 64 | 2 MiB | success ×3 | drop / exit 0 |
| release | 默认边界 | chain | 64 | 64 | 8 MiB | success ×3 | drop / exit 0 |
| release | 默认边界 | chain | 65 | 64 | 2 MiB | depth-exceeded ×3 | drop / exit 0 |
| release | 默认边界 | chain | 65 | 64 | 8 MiB | depth-exceeded ×3 | drop / exit 0 |
| release | 提高深度 | chain | 256 | 256 | 2 MiB | success ×3 | drop / exit 0 |
| release | 提高深度 | chain | 256 | 256 | 8 MiB | success ×3 | drop / exit 0 |
| release | 提高深度 | chain | 256 | 255 | 2 MiB | depth-exceeded ×3 | drop / exit 0 |
| release | 提高深度 | chain | 256 | 255 | 8 MiB | depth-exceeded ×3 | drop / exit 0 |
| release | 提高深度 | chain | 1024 | 1024 | 2 MiB | success ×3 | drop / exit 0 |
| release | 提高深度 | chain | 1024 | 1024 | 8 MiB | success ×3 | drop / exit 0 |
| release | 提高深度 | chain | 1024 | 1023 | 2 MiB | depth-exceeded ×3 | drop / exit 0 |
| release | 提高深度 | chain | 1024 | 1023 | 8 MiB | depth-exceeded ×3 | drop / exit 0 |
| release | 提高深度 | chain | 4096 | 4096 | 2 MiB | success ×3 | drop / exit 0 |
| release | 提高深度 | chain | 4096 | 4096 | 8 MiB | success ×3 | drop / exit 0 |
| release | 提高深度 | chain | 4096 | 4095 | 2 MiB | depth-exceeded ×3 | drop / exit 0 |
| release | 提高深度 | chain | 4096 | 4095 | 8 MiB | depth-exceeded ×3 | drop / exit 0 |
| release | 提高深度 | chain | 8192 | 8192 | 2 MiB | stack-overflow ×3 | read / SIGABRT |
| release | 提高深度 | chain | 8192 | 8192 | 8 MiB | success ×3 | drop / exit 0 |
| release | 提高深度 | chain | 8192 | 8191 | 2 MiB | stack-overflow ×3 | read / SIGABRT |
| release | 提高深度 | chain | 8192 | 8191 | 8 MiB | depth-exceeded ×3 | drop / exit 0 |
| release | 长循环 | cycle | 256 | 257 | 2 MiB | cycle ×3 | drop / exit 0 |
| release | 长循环 | cycle | 256 | 257 | 8 MiB | cycle ×3 | drop / exit 0 |
| release | 长循环 | cycle | 1024 | 1025 | 2 MiB | cycle ×3 | drop / exit 0 |
| release | 长循环 | cycle | 1024 | 1025 | 8 MiB | cycle ×3 | drop / exit 0 |

## 实施命令与结果

| 检查 | 结果 | 备注 |
| --- | --- | --- |
| 初始命令 `cargo fmt --all -- --check && cargo build --locked --example config_interpolation_resource_probe` | exit 1 | `cargo fmt` 阶段 Cargo metadata 无法读取 `../rs-redact` 解析到的缺失路径 `/data/working/qubit/rust-common/.worktrees/rs-config/rs-redact/Cargo.toml`；因 `&&` 前半失败，build 未执行。随后单独 build 同样无法读取该路径并 exit 101。 |
| `./.infra/bin/prepare-local-path-dependencies.sh` | exit 0 | helper 成功，无 local path dependency 配置项，未生成链接。 |
| 临时依赖链接尝试后的 `cargo fmt --all -- --check` | exit 1 | 在容器 checkout 中建立的短暂链接使 Cargo 将 `rs-value` 误判为外层 rs-config workspace 的非成员；链接已移除，未改任何依赖仓库。 |
| 首次实际编译 `cargo build --locked --example config_interpolation_resource_probe` | exit 101 | 临时改用目标 worktree 内 Cargo.toml 绝对依赖路径后开始编译；发现 `E0521`（借用的 mode 跨线程逃逸）和 `E0004`（成功值匹配不穷尽）。 |
| 第 2 次 debug build | exit 101 | 修正 mode 生命周期和显式 cycle 成功分支后，仍发现 `E0004`：`&str` 匹配对编译器不封闭。再增加通用意外成功分支。 |
| 修正后的 `cargo build --locked --example config_interpolation_resource_probe` | exit 0 | debug profile。为适配嵌套 worktree 的相对路径依赖布局，运行期间临时将目标 worktree `Cargo.toml` 的两个依赖路径改为现存原 checkout 绝对路径；命令完成后原样恢复。 |
| 修正后的 `cargo build --locked --release --example config_interpolation_resource_probe` | exit 0 | release profile，采用同一临时路径处理并恢复。 |
| Cargo.toml / Cargo.lock 哈希核对 | 通过 | `Cargo.toml` 原始和最终 SHA-256：`166c4051bb25bc516ef4809b6e4975b97ee4825319023fc0494702fee57fa6ef`；`Cargo.lock` 原始和最终 SHA-256：`3b0f1c013bb4a14df68477aadd1bd55a33e7118c0af43b0c7a18c32ea709594c`。无依赖路径或锁文件差异。 |
| `cargo fmt --all -- --check` | exit 1（非 T1 门禁） | 临时路径解析后，该宽泛检查也扫描本地路径依赖及既有 bench 文件，报告这些文件已有的格式差异；命令为只读，未格式化或修改它们。探测 example 单文件 `rustfmt --edition 2024 --check examples/config_interpolation_resource_probe.rs` 通过。 |
| Python 探测脚本语法解析 | exit 0 | `ast.parse` 检查。 |
| `python3 -B scripts/probe-interpolation-resources.py --profile debug --output target/interpolation-probe/debug` | exit 0 | 90 次：57 个预期结果，33 次归因到 stack overflow；无 timeout、OOM、driver error。 |
| `python3 -B scripts/probe-interpolation-resources.py --profile release --output target/interpolation-probe/release` | exit 0 | 90 次：84 个预期结果，6 次归因到 stack overflow；无 timeout、OOM、driver error。 |
| `python3 -B scripts/check-doc-examples.py` | exit 0 | `checked 12 canonical example IDs in 4 documents`。 |

## 风险归因与后续判定依据

每次 stack-overflow 运行都有 `phase=construct`、`phase=read` 标记；stdout 最后的完整标记为 `phase=read`，stderr 为 Rust 明确的 `has overflowed its stack` 和 `fatal runtime error: stack overflow, aborting`，父进程记录负退出码对应 `SIGABRT`。所有对应探测均在 10 秒内结束；没有 `RLIMIT_AS` 设置失败或内存分配错误。R=1024 循环在 8 MiB（debug）或两种 release 栈上返回并断言循环错误，说明较小栈的异常由递归展开深度造成，而非循环错误被测试逻辑误认为栈故障。

本证据满足设计中的风险确认条件，可以进入 T2 条件修复分支：先挑选矩阵中最小且稳定的失败配置建立隔离回归，然后评估保持行为的显式帧栈修复。T1 未修改生产插值算法、用户指南、设计文档或下游仓库；本报告不宣称任意深度安全。

## T2 RED：修复前隔离回归

修复前新增 `tests/interpolation_stack_tests.rs`，以测试二进制通过 `--exact` 启动唯一子用例；子用例在独立子进程的 2 MiB 线程中构造 R=1024 链并读取，父进程 10 秒轮询，超时会 kill 并 wait 回收。该点取自 T1 的最小稳定失败配置。测试宿主 core limit 设为 0。

| 命令 | 退出 | 证据 |
| --- | ---: | --- |
| `cargo test --locked --test interpolation_stack_tests interpolation_1024_deep_chain_fits_2mib_stack -- --exact --nocapture` | 101（预期 RED） | 子进程 `ExitStatus(unix_wait_status(134))`；stdout 到达 `phase=construct`、`phase=read`，stderr 为 `has overflowed its stack` 和 `fatal runtime error: stack overflow, aborting`。父测试因子进程异常退出而失败；不是编译错误、超时或构造错误。 |

RED 运行 stdout/stderr 保存在 `target/interpolation-probe/t2-red/`，不覆盖 T1 原始矩阵日志。运行期间使用 T1 核验过的临时 Cargo 依赖路径映射；Cargo.toml 与 Cargo.lock SHA-256 均保持不变。此回归在生产实现修改之前真实失败。

## T2：显式帧栈修复与复测

展开现在由 `src/utils/internal/interpolation_frame.rs` 中的拥有型帧保存输入、扫描游标、已追加位置、部分输出和进入变量名；`substitute_variables_by` 使用 `Vec<InterpolationFrame>` 和原有 `Vec<String>` 活动链执行迭代。根输入只复制一次，不按 `max_depth` 预分配帧；变量解析闭包、`InterpolationSession` 和错误类型未变。纯文本帧仍先检查输出，再退出活动变量；含占位符帧仍先做深度检查。每个占位符的执行顺序保持为受限追加前缀、检查循环、扣展开预算、resolve；帧尾受限追加，子帧完成时先弹活动变量，再受限归并父帧。

帧栈初版的 focused 测试发现了一个额外回归：纯文本子帧快速完成路径没有像复杂帧路径那样弹出进入变量。既有的 `${name}-${name}-${name}` 测试在该初版上失败，得到 `SubstitutionCycle { path: "value", chain: ["name", "name"] }`；为保持计划顺序，先检查输出，再从快速路径弹出活动变量，归并结果。最终该既有用例通过。首次 focused build 还发现内部帧 re-export 可见性（E0365/E0603）及活动链 Vec 类型推断（E0282）；分别通过限制到 `crate::utils` 可见性和声明 `Vec<String>` 修正。一次短名 `--exact test_get_string_substitutes_repeated_placeholder` 只筛出 0 个测试（Cargo exit 0，91 filtered）；该结果作废，随后使用完整模块名重跑并确实得到上述 RED。

`tests/interpolation_stack_tests.rs` 现有两个行为回归均通过测试二进制的 `--exact` 子用例在独立进程中运行；父进程使用 `try_wait` 轮询、10 秒后 kill/wait。子用例在 2 MiB 线程内构造 R=1024 成功链，或构造由 1024 个变量组成的闭合环并断言完整 `SubstitutionCycle` path/chain。每个危险子进程在构造输入前通过 Unix `setrlimit(RLIMIT_CORE, 0)` 关闭 core；Linux 下测试断言实际设置成功。非 Unix 或设置失败时打印限制不可用状态并继续运行行为断言，不把 skipped 作为通过结果。T1 Python 驱动也改为可选资源限制：仅在 POSIX/resource 可用时用子进程 pre-exec 设置；每个子进程将 RLIMIT_AS 与 RLIMIT_CORE 的请求值、实际值和状态单独写入 `resource-limits.json` 并汇总到 CSV；不支持平台仍执行样例并记录 `unsupported`，不会伪报已限额。

计划要求的语义断言由现有及新增用例覆盖：成功占位符前缀/后缀（`Hello, ${name}!`、`http://${host}:${port}/api`）、重复同一纯文本变量非循环、UTF-8 按字节超限、未解析变量保留读取 path、完整闭合循环链、默认 R=63/64 成功与 R=65 深度错误、提高限额 R=L 成功与 R=L+1 深度错误。显式插值测试继续覆盖 section 根回退、环境变量允许/拒绝、结构化字段累计预算和集合索引/path。

成功修复后先运行过 `target/interpolation-probe/post-fix/{debug,release}` 矩阵。最终在加入 root plain-text 快速检查后，又以不覆盖旧 run ID 的方式将全矩阵运行到 `target/interpolation-probe/post-fix-final/{debug,release}`；后者是本次最终代码的结果。每个 profile 均有 90 次、30 个组合每点三次，所有点为预期结果；分类为 48 次成功、30 次深度超限、12 次闭环循环；没有栈溢出、超时、构造/读取异常或驱动错误。180 次子进程的 `RLIMIT_AS=512 MiB` 与 `RLIMIT_CORE=0` 状态全部为 `applied`，每次均有 JSON 实际值记录。原 pre-fix 矩阵共 39 次由递归展开触发的 SIGABRT stack overflow；最终 post-fix 同矩阵为 0。R=8192、2 MiB 栈的最终 debug 最长单点约 0.81 秒，release 约 0.19 秒，未触及 10 秒超时。结果只适用于已测平台和矩阵，不承诺任意深度的 RSS 或 CPU 上限。

### T2 命令与退出码

所有 Cargo 命令仍通过目标 worktree 内临时路径映射读取原 rs-redact/rs-value checkout；每次结束均恢复 `Cargo.toml`，并核对 `Cargo.lock` 未变。重复核验哈希为：Cargo.toml `166c4051bb25bc516ef4809b6e4975b97ee4825319023fc0494702fee57fa6ef`，Cargo.lock `3b0f1c013bb4a14df68477aadd1bd55a33e7118c0af43b0c7a18c32ea709594c`。

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| 修复前 `cargo test --locked --test interpolation_stack_tests interpolation_1024_deep_chain_fits_2mib_stack -- --exact --nocapture` | 101（预期 RED） | 子进程 exit 134；read 阶段 stderr 明确为 stack overflow。 |
| 帧栈初版 `cargo test --locked --test utils_tests test_variable_substitution::test_get_string_substitutes_repeated_placeholder -- --exact --nocapture` | 101（预期 RED） | 初版快速路径漏弹 active variable，误报 `name -> name` 循环；修复该路径后同命令 exit 0。 |
| `cargo test --locked --test interpolation_stack_tests -- --nocapture` | 0 | 2/2 通过；每个 Linux 子进程均断言 `core_limit=applied`，链和闭环都在 2 MiB 栈完成。 |
| `cargo test --locked --test utils_tests test_variable_substitution -- --nocapture` | 0 | 32 passed，0 failed，含 UTF-8、预算、变量重复、section/environment、默认与提高深度边界。 |
| `cargo test --locked --test explicit_interpolation_tests` | 0 | 28 passed，0 failed；结构化累计会话、section 与 environment 契约通过。 |
| `cargo build --locked --example config_interpolation_resource_probe` | 0 | 修复后 debug build。 |
| `cargo build --locked --release --example config_interpolation_resource_probe` | 0 | 修复后 release build。 |
| `python3 -B scripts/probe-interpolation-resources.py --profile debug --output target/interpolation-probe/post-fix/debug` | 0 | 90/90 预期，48 success / 30 depth-exceeded / 12 cycle；两项 RLIMIT 全部生效。 |
| `python3 -B scripts/probe-interpolation-resources.py --profile release --output target/interpolation-probe/post-fix/release` | 0 | 90/90 预期，48 success / 30 depth-exceeded / 12 cycle；两项 RLIMIT 全部生效。 |
| `python3 -B scripts/probe-interpolation-resources.py --profile debug --output target/interpolation-probe/post-fix-final/debug` | 0 | 最终源码 90/90 预期；48 success / 30 depth-exceeded / 12 cycle；90/90 各自 RLIMIT_AS 与 RLIMIT_CORE 均 applied，逐次 JSON 记录。 |
| `python3 -B scripts/probe-interpolation-resources.py --profile release --output target/interpolation-probe/post-fix-final/release` | 0 | 最终源码 90/90 预期；48 success / 30 depth-exceeded / 12 cycle；90/90 各自 RLIMIT_AS 与 RLIMIT_CORE 均 applied，逐次 JSON 记录。 |
| Python 探测驱动 `ast.parse` 语法检查 | 0 | 可选资源限制分支语法有效。 |
| `python3 -B scripts/check-doc-examples.py` | 0 | 12 canonical example IDs / 4 documents。 |
| 单文件 `rustfmt --edition 2024 --check`（修改后的实现与测试文件；`mod.rs` 使用 `skip_children=true`） | 0 | 修改代码的定点格式检查通过。 |
| 对 `src/reader/read_policy_builder.rs` 的 `rustfmt --edition 2024 --check` | 1 | 只读检查发现该文件已有的大量基线格式差异；未对文件写盘格式化。 |
| `git diff --check` | 0 | 无空白错误。 |

T2 已按证据进入条件修复分支：公共 API、预算默认值/边界、错误 path/category、查找和 fallback 逻辑均保持；未改下游 crate、Cargo.toml/Cargo.lock 路径依赖。

## T3 独立复查与仓库验证

独立只读复查未发现插值帧顺序、预算语义、错误处理或新增测试方面的代码缺陷；复查结论要求继续完成下游契约和完整 CI。本任务 worktree 因 `../rs-redact`、`../rs-value` 相对路径依赖最初无法从 `.worktrees/rs-config/...` 解析，使用 Git `worktree move` 移至仓库根下的同级目录 `.rs-config-interpolation-stack-safety-20261104`，使同一 workspace 中既有依赖 checkout 可正常解析。`cargo metadata --locked --format-version 1 --no-deps --manifest-path /data/working/qubit/rust-common/.rs-config-interpolation-stack-safety-20261104/Cargo.toml` exit 0；没有修改依赖仓库或 Cargo manifest/lock。

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `./.infra/bin/align-ci.sh` | 0 | 仓库格式/风格对齐步骤通过；之后重新运行相关测试。 |
| `cargo test --locked --test utils_tests test_variable_substitution -- --nocapture` | 0 | 对齐后重跑，32 passed。 |
| `cargo test --locked --test interpolation_stack_tests -- --nocapture` | 0 | 对齐后重跑，2 passed。 |
| `cargo test --locked --test downstream_contract_tests` | 0 | 11 passed。 |
| `cargo test --locked --all-features --test downstream_contract_tests` | 0 | 11 passed。 |
| `./.infra/bin/ci-check.sh` | 0 | 完整项目 CI 通过，包括 style、clippy、verify/测试与文档测试、strict-doc、readme、release build、feature matrix、fuzz smoke、project hook、package、coverage 和 audit。coverage 门槛均满足：functions 99.34%（≥95%）、lines 97.24%（>90%）、regions 95.93%（>85%）。仓库配置 `coverage_cfg_clippy=false`，脚本输出 `coverage-cfg-clippy: not configured; skipping`；这是项目显式配置的跳过项。两个 fuzz smoke 及包校验、漏洞审计均成功。 |
| `git diff --check` | 0 | 无空白错误。 |

CI 结束时脚本清理了自己的 `target` 临时构建产物；fuzz smoke 在忽略目录 `fuzz/corpus/config_wire` 新生成了 corpus 样本，提交后将只清理可确认由本轮产生的未跟踪样本并保留原有三个跟踪种子。下游契约验证只运行只读测试，没有修改相关下游仓库。
