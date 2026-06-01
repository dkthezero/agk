# AGK 支持指南

> **AGK**（Agent Kit）是一款终端工具，用于统一管理多种 AI 编码助手的技能、指令和 MCP 服务器配置——一处配置，处处生效。

`[Personal]` `[Team]` `[Org]` 徽章标示各章节的主要适用用户类型。所有用户均可使用所有功能；徽章仅用于突出主要受众。

---

## 目录

1. [概述](#1-概述)
2. [安装](#2-安装)
3. [核心概念](#3-核心概念)
4. [快速上手](#4-快速上手)
5. [分步指南](#5-分步指南)
6. [团队与组织指南](#6-团队与组织指南)
7. [TUI 参考](#7-tui-参考)
8. [CLI 参考](#8-cli-参考)
9. [配置参考](#9-配置参考)
10. [故障排除](#10-故障排除)
11. [Provider 专属指南](#11-provider-专属指南)

---

## 1. 概述

你可能同时使用多款 AI 编码工具——Claude Code、GitHub Copilot、Gemini CLI、OpenCode 等等。每款工具有各自的目录结构、配置格式和添加技能与指令的方式。手动保持它们同步既繁琐又容易出错。

**AGK 解决了这个问题。** 你可以把它想象成一个控制中心，将你的 AI 代理配置一次性广播到所有 provider：

```
  ┌─────────┐     ┌─────────┐     ┌─────────┐
  │  Local   │     │ GitHub  │     │ ClawHub │
  │  Vault   │     │  Vault  │     │ Market  │
  └────┬─────┘     └────┬────┘     └────┬─────┘
       │                │                │
       └────────────────┼────────────────┘
                        │
                   ┌────┴────┐
                   │   AGK    │
                   └────┬────┘
        ┌──────┬──────┬──────┼──────┬──────┬──────┬──────┐
        │      │      │      │      │      │      │      │
    Claude  Open  GitHub  Gemini  AMP  Fire-  Letta  Snow-
    Code    Code  Copilot  CLI        bender         flake
```

**AGK 的功能：**
- **Skills（技能）** —— 可复用的工具，让你的 AI 代理更聪明（就像在手机上安装应用）
- **Instructions（指令）** —— 行为规则，塑造 AI 代理的响应方式（类似系统偏好设置）
- **MCP Servers（MCP 服务器）** —— 连接 AI 与外部服务的桥梁（类似浏览器扩展）
- **Profiles（配置方案）** —— 将 provider、技能、MCP 和权限打包在一起的命名配置（类似主屏幕布局）
- **Vaults（仓库）** —— 技能、指令和配置方案的来源（类似应用商店）
- **Contexts（上下文）** —— 可切换的工作区，适用于不同团队或项目（类似电脑上的用户账户）

**支持的 provider：** Claude Code · OpenCode · GitHub Copilot · Gemini CLI · AMP Code · Firebender · Letta · Snowflake Cortex

`[Personal]` `[Team]` `[Org]`

---

## 2. 安装

### Homebrew（macOS 和 Linux）

```bash
brew tap agk/tap
brew install agk
```

### Cargo（从源码构建）

```bash
cargo install agk
```

需要较新版本的 Rust 工具链。如未安装，请从 [rustup.rs](https://rustup.rs/) 安装 Rust。

### 预编译二进制文件

从 [GitHub Releases](https://github.com/agk-cli/agk/releases) 下载并添加到你的 `PATH` 中。

### 验证安装

```bash
agk --version
```

如果显示了版本号，说明安装成功。

`[Personal]` `[Team]`

---

## 3. 核心概念

### 3.1 Skills（技能）

**Skill（技能）** 是 AI 代理可以使用的工具。它是一个包含 `SKILL.md` 文件和可选子目录（`scripts/`、`references/`、`assets/`）的文件夹。

你可以把 skill 想象成手机上的**应用**——安装后便增加了一项能力。

```
my-vault/
  skills/
    web-browser/
      SKILL.md          # 必需——描述该技能
      scripts/           # 可选——可执行脚本
      references/        # 可选——参考文档
      assets/            # 可选——附加文件
```

安装技能时，AGK 会将其复制到 provider 的技能目录（例如，Claude Code 对应 `~/.claude/skills/web-browser/`）。

`[Personal]` `[Team]`

### 3.2 Instructions（指令）

**Instruction（指令）** 是针对 AI 代理的行为规则。它是一个包含 `AGENTS.md` 文件的文件夹。

你可以把 instruction 想象成**系统偏好设置**——它塑造行为而非添加工具。例如，一条指令可能规定"始终先写测试"或"使用要点列表回复"。

指令会被安装到各 provider 的专属目录（例如，Claude Code 在工作区作用域下对应 `.claude/instructions/my-rule/`）。

`[Personal]` `[Team]`

### 3.3 MCP Servers（MCP 服务器）

**MCP server（MCP 服务器）**（Model Context Protocol）是 AI 代理与外部服务之间的桥梁——数据库、文件系统、浏览器、API 等。

你可以把 MCP 服务器想象成**浏览器扩展**——插入后即可为 AI 增加它原本没有的能力。

MCP 服务器支持两种传输方式：
- **stdio** —— 服务器作为本地进程运行。AGK 启动它并通过标准输入/输出通信。
- **SSE** —— 服务器作为远程 HTTP 服务运行。AGK 连接到一个 URL。

注册 MCP 服务器时，AGK 将其存储在 `~/.config/agk/mcp.toml` 中，并可以按 provider 和作用域启用。

> **提示：** 注册 MCP 服务器后，AGK 会自动运行握手测试以验证连接。`[✓]` 徽章表示测试通过。

`[Personal]` `[Team]`

### 3.4 Providers（提供方）

**Provider（提供方）** 是技能和指令最终部署到的 AI 平台。你可以把它想象成你的**手机**——AGK 把你的"应用"安装到你选择的所有手机上。

| Provider | ID | Skills | Instructions | MCP | Profiles | Config roots |
|---|---|---|---|---|---|---|
| Claude Code | `claude-code` | ✓ | ✓ | ✓ | ✓ | `.claude`, `.agents` |
| OpenCode | `opencode` | ✓ | ✓ | ✓ | ✓ | `.opencode`, `.agents` |
| GitHub Copilot | `github-copilot` | ✓ | ✓ | ✓ (global only) | — | — |
| Gemini CLI | `gemini-cli` | ✓ | ✓ | ✓ (global only) | — | `.gemini`, `.ai` |
| AMP Code | `amp` | ✓ | ✓ | ✓ | — | — |
| Firebender | `firebender` | ✓ | ✓ | — | — | — |
| Letta | `letta` | ✓ | ✓ | — | — | — |
| Snowflake Cortex | `snowflake` | ✓ | ✓ | — | — | — |

某些 provider（Claude Code、OpenCode、Gemini CLI）允许你选择 **config root** —— 即存储技能和指令的文件夹名称。例如，OpenCode 可以使用 `.opencode`（默认）或 `.agents`（与 Claude 兼容）。你在首次激活 provider 时进行选择。

`[Personal]` `[Team]`

### 3.5 Vaults（仓库）

**Vault（仓库）** 是技能的来源。你可以把它想象成**应用商店**——你挂载一个仓库，浏览其中的内容，然后安装你需要的东西。

AGK 支持三种仓库类型：

| 仓库类型 | 工作方式 | 示例 |
|---|---|---|
| **Local** | 本地磁盘上的目录 | `./my-vault` |
| **GitHub** | GitHub 仓库（稀疏检出） | `owner/repo` |
| **ClawHub** | ClawHub 社区市场 | 内置，切换即可激活 |

对于 GitHub 仓库，AGK 使用 `git sparse-checkout` 仅获取你需要的子目录，保持速度快捷。你可以指定分支（默认：`main`）和子目录路径（默认：`skills/`）。

对于 ClawHub，AGK 使用 `clawhub` CLI 来搜索和安装社区包。如果未安装该 CLI，AGK 会提供通过 Homebrew 安装或手动下载链接的选项。

`[Personal]` `[Team]` `[Org]`

### 3.6 Profiles（配置方案）

**Profile（配置方案）** 是一个命名的、自包含的配置，将 provider 与选定的技能、MCP、指令和权限设置打包在一起。你可以把它想象成**主屏幕布局**——同一部手机，工作用和个人用的应用排列各不相同。

启动一个 profile 时，AGK 会：
1. 生成包含你选定工具和权限的代理 markdown 文件
2. 将 MCP 服务器和技能权限写入 provider 的配置
3. 启动 provider CLI
4. 会话结束时清理所有更改

通过 TUI 向导创建 profile（在 Profiles 标签页按 `F2`）或使用 CLI（`agk profile create`）。

**Profile 向导预设模板：**

| 预设模板 | 角色 | 默认工具 | 权限模式 |
|---|---|---|---|
| Code Reviewer | 高级代码审查员 | Read, Glob, Grep, LSP | default |
| Feature Implementer | 高级工程师 | Read, Glob, Grep, Bash, Write, Edit | default |
| Security Auditor | 安全工程师 | Read, Glob, Grep, Bash | default |
| Documentation Writer | 技术文档工程师 | Read, Glob, Grep, Write, Edit | default |
| Test Generator | 测试工程师 | Read, Glob, Grep, Bash, Write | default |
| Custom | 空白模板 | — | — |

**权限模式：**

| 模式 | 行为 |
|---|---|
| `default` | 编辑前请求确认 |
| `acceptEdits` | 自动接受编辑 |
| `auto` | 自动批准安全操作 |
| `dontAsk` | 从不请求确认 |
| `plan` | 计划模式——仅建议，不执行 |

`[Personal]` `[Team]`

### 3.7 Contexts（上下文）

**Context（上下文）** 是一个可命名的、可切换的工作区。你可以把它想象成电脑上的**用户账户**——一个用于个人项目，一个用于公司项目，一个用于特定客户。

每个上下文包含自己的：
- 显示名称（例如，"Personal"、"Acme Corp"、"Client X"）
- Vault 列表
- Provider 列表
- Profile 列表
- 环境标签（local、dev、staging、prod）
- 标签（键值对）

默认上下文名为 `default`，显示名称为"Personal"。切换上下文时，会将该上下文的仓库和 provider 合并到你的活跃配置中。

```bash
agk context list               # 显示所有上下文
agk context switch acme-corp   # 切换到 acme-corp 上下文
agk context create client-x --display-name "Client X"  # 创建新上下文
```

`[Team]` `[Org]`

### 3.8 Scope（作用域：全局 vs 工作区）

**Scope（作用域）** 决定 AGK 在哪里存储配置和已安装的资源。

| 作用域 | 配置路径 | 用途 |
|---|---|---|
| **Global** | `~/.config/agk/config.toml` | 仓库定义、provider 激活、全局 profile |
| **Workspace** | `.agk/config.toml` | 当前项目的已安装资源 |

你可以把它想象成**系统设置 vs 应用设置**——全局作用域对你机器上的所有项目生效，工作区作用域仅对当前项目文件夹生效。

在 TUI 中，按 `Tab` 切换作用域。在 CLI 中，使用 `--scope global` 或 `--scope workspace`。

> **注意：** 仓库和 provider 通常在全局作用域中配置。已安装的资源（技能、指令）通常在工作区作用域中跟踪。

`[Personal]` `[Team]`

### 3.9 SHA10 变更检测

AGK 使用 **SHA10** 来追踪已安装的资源是否为最新版本——对每个资源进行内容指纹校验。它对技能的 `SKILL.md` 及其 `scripts/`、`references/`、`assets/` 目录进行哈希计算，取前 10 个字符。

当已安装的 SHA10 与扫描到的 SHA10 一致时，资源显示为**最新**。如果仓库中的技能被更新了，即使版本号没有变化，SHA10 也会改变，因此 AGK 始终能知道何时需要更新。

在 TUI 中，按 `Enter` 更新单个过时资源，或按 `F5` 一次性更新所有内容。

`[Personal]` `[Team]`

### 3.10 Meta-skills 和依赖关系

**Meta-skill（元技能）** 是一种技能，其 `SKILL.md` 前置数据中列出了其他技能作为依赖。你可以把它想象成一个**套装包**或**元包**——安装它会同时安装它所需的一切。

```yaml
# SKILL.md 前置数据
---
name: company-onboarding-pack
version: "1.0.0"
requires:
  - clawhub/git-workflow
  - clawhub/code-review
requires_optional:
  - clawhub/security-audit
---
```

- `requires` —— 始终会安装的依赖。
- `requires_optional` —— 用户可以选择跳过的依赖。

AGK 会递归解析依赖。如果两个元技能依赖同一个技能，该技能只会被安装一次（菱形依赖去重）。循环依赖会被检测到并以错误提示拒绝。

`[Team]`

---

## 4. 快速上手

按照本节的步骤，你可以在 5 分钟内从零开始搭建完成。

### 4.1 启动 TUI

```bash
agk
```

你会看到一个全屏终端界面，顶部有标签页，底部有快捷键提示。

### 4.2 挂载仓库

1. 按 `0` 切换到 **Vaults** 标签页。
2. 按 `F2` 挂载新仓库。
3. 输入本地路径（例如 `./my-vault`）或 GitHub URL（例如 `my-org/team-skills`）。
4. 对于 GitHub 仓库：确认分支（默认 `main`）和子目录路径（默认 `skills/`）。
5. 输入仓库名称（默认为文件夹名或仓库名）。

你也可以在 Vaults 标签页中按 `Space` 选中 `clawhub` 条目来激活内置的 ClawHub 仓库。

### 4.3 激活 provider

1. 按 `4` 切换到 **Providers** 标签页。
2. 在你想激活的 provider 上按 `Space`（例如 `claude-code`）。
3. 如果该 provider 支持多个 config root，从弹窗中选择一个。

### 4.4 安装第一个技能

1. 按 `1` 切换到 **Skills** 标签页。
2. 输入关键词搜索技能。
3. 在你想安装的技能上按 `Space`。
4. AGK 将技能文件复制到 provider 的技能目录，并在 `config.toml` 中记录安装信息。

### 4.5 注册 MCP 服务器

1. 按 `2` 切换到 **MCP** 标签页。
2. 按 `F2` 启动注册向导。
3. 填写 5 个步骤：**Name**、**Command**、**Arguments**、**Transport**（stdio 或 SSE）、**Description**。
4. AGK 会自动运行握手测试。如果成功，你会看到 `[✓]` 徽章。

### 4.6 创建 profile

1. 按 `5` 切换到 **Profiles** 标签页。
2. 按 `F2` 启动 profile 向导。
3. 按照步骤操作：名称、作用域、预设模板、身份问题、技能清单、MCP 清单、工具/权限选择、审核。
4. 启动 profile：

```bash
agk profile start my-profile
```

`[Personal]`

---

## 5. 分步指南

### 5.1 管理仓库

**挂载本地仓库：**

```bash
# 通过 TUI：按 0 → F2 → 输入路径 → 输入名称
# 或通过配置文件：
```

```toml
# ~/.config/agk/config.toml
[my-vault.vault]
type = "local"
path = "/path/to/my-vault"
```

**挂载 GitHub 仓库：**

```bash
# 通过 TUI：按 0 → F2 → 输入 "owner/repo" → 确认分支 → 确认路径 → 输入名称
# 或通过配置文件：
```

```toml
[team-skills.vault]
type = "github"
repo = "my-org/team-skills"
ref = "main"
path = "skills/"
```

**激活 ClawHub：**
- 在 TUI 中，导航到 Vaults 标签页，在 `clawhub` 条目上按 `Space`。
- 如果未安装 `clawhub` CLI，AGK 会提供通过 Homebrew 安装或手动下载链接的选项。

**卸载仓库：**
- 在 TUI 中，导航到 Vaults 标签页，选中仓库，按 `Space` 切换为关闭状态。会弹出确认对话框。

**刷新仓库：**
- 在任意标签页按 `F4` 从源头刷新所有仓库。

`[Personal]` `[Team]`

### 5.2 安装和更新技能

**安装技能：**

```bash
# TUI：按 1 → 输入搜索 → Space 安装
# CLI：
agk install web-browser
agk install my-vault/web-browser       # 从指定仓库
agk install web-browser:1.2.0          # 指定版本
```

**更新单个技能：**
- 在 TUI 中，选中技能后按 `Enter`。

**更新所有技能：**
- 在任意标签页按 `F5`。

**安装时包含评估用例：**

```bash
agk install web-browser --evals
```

`--evals` 标志会在安装时包含 `evals/` 子目录（测试用例）。

`[Personal]` `[Team]`

### 5.3 使用指令

指令的操作方式与技能相同——`Space` 安装，`Enter` 更新，`F5` 批量更新。区别在于内容：指令包含 `AGENTS.md` 行为提示词，而非 `SKILL.md` 工具定义。

各 provider 的指令安装路径参见[第 11 节](#11-provider-专属指南)。

`[Personal]` `[Team]`

### 5.4 MCP 服务器管理

**注册 MCP 服务器：**

```bash
# TUI：按 2 → F2 → 填写 5 个步骤
# CLI：
agk mcp add \
  --name my-server \
  --command "npx" \
  --args "-y,@modelcontextprotocol/server-filesystem,/tmp" \
  --transport stdio \
  --description "Filesystem access server"
```

**为 provider 启用 MCP 服务器：**

```bash
agk mcp enable my-server --provider claude-code
agk mcp enable my-server --provider claude-code --scope global
```

**禁用 MCP 服务器：**

```bash
agk mcp disable my-server --provider claude-code
```

**列出已注册的 MCP 服务器：**

```bash
agk mcp list
agk mcp list --provider claude-code
```

**测试 MCP 服务器连接：**

```bash
agk mcp test my-server
```

> **警告：** MCP 握手测试会在你的机器上运行服务器命令。请只注册你信任的 MCP 服务器。

> **注意：** CLI 命令 `agk mcp add` 不支持直接指定 SSE URL。要注册 SSE 服务器，请使用 TUI 向导（MCP 标签页 → `F2`）或直接编辑 `~/.config/agk/mcp.toml`，设置 `transport = "sse"` 并填写 `url` 字段。

`[Personal]` `[Team]`

### 5.5 创建和启动 Profile

**通过 TUI 向导创建（推荐）：**
- 按 `5` → `F2` → 按照多步向导操作。

**通过 CLI 创建：**

```bash
agk profile create my-reviewer \
  --provider claude-code \
  --skills "code-reviewer,security-audit" \
  --mcps "my-server" \
  --description "Reviews code for quality and security" \
  --scope workspace
```

**启动 profile：**

```bash
agk profile start my-reviewer
```

**预览但不实际运行：**

```bash
agk profile start my-reviewer --dry-run
```

这会显示启动计划（将创建哪些文件、将修改哪些配置），但不会实际启动会话。

`[Personal]` `[Team]`

### 5.6 切换上下文

```bash
# 列出所有上下文
agk context list

# 切换上下文
agk context switch acme-corp

# 创建新上下文
agk context create client-x --display-name "Client X"
```

切换上下文时，AGK 将新上下文的仓库和 provider 合并到你的活跃全局配置中。上一个上下文的增量会先被移除。

上下文存储在 `~/.config/agk/contexts.toml` 中。

`[Team]` `[Org]`

### 5.7 应用声明式配置（团队入职）

`agk apply` 读取一个配置源（URL 或文件），并将你的本地设置调整为与其一致。你可以把它想象成 AI 工具领域的 `docker compose up` ——你声明想要的状态，`apply` 让它变为现实。

```bash
# 从 URL 应用
agk apply https://raw.githubusercontent.com/my-org/configs/main/team.toml

# 从本地文件应用
agk apply ./team-config.toml

# 预览但不实际修改
agk apply ./team-config.toml --dry-run

# 应用到指定上下文和环境
agk apply ./team-config.toml --context acme-corp --environment prod
```

配置源可以指定仓库、provider、profile 和 MCP 服务器。`agk apply` 会添加缺失的条目、更新有变化的条目，并移除配置源中已不存在的条目。

`[Team]` `[Org]`

### 5.8 同步资源

```bash
# 同步所有已配置的资源（安装缺失的，更新过时的）
agk sync

# 在全局作用域中同步
agk sync --global

# 预览但不实际修改
agk sync --dry-run
```

`[Personal]` `[Team]`

### 5.9 打包技能以供分发

```bash
# 为 Claude Desktop 打包
agk pack web-browser --target claude-desktop

# 打包为 tarball
agk pack web-browser --target tarball

# 输出到 stdout（支持管道）
agk pack web-browser --target tarball --stdout > my-skill.tar.gz
```

打包目标：`claude-desktop`、`firebender`、`tarball`。

`[Team]`

### 5.10 遥测与使用分析

AGK 仅在本地收集遥测数据——不会向外部发送任何数据。数据存储在 `~/.config/agk/analytics.toml` 中。

```bash
agk telemetry status            # 检查遥测是否启用
agk telemetry enable            # 启用遥测
agk telemetry disable           # 禁用遥测
agk telemetry export            # 导出为 JSON（默认）
agk telemetry export --format csv   # 导出为 CSV
agk telemetry export --output ~/analytics.json  # 写入文件
```

`[Personal]` `[Team]`

### 5.11 清理

```bash
# 移除工作区配置
agk clean

# 移除全局配置
agk clean --global
```

> **警告：** `agk clean` 会移除配置文件。provider 目录中已安装的技能文件不会被移除——只有 AGK 配置会被删除。

`[Personal]`

---

## 6. 团队与组织指南

### 6.1 使用 Apply 进行团队入职

让新团队成员快速上手的最佳方式是使用 `agk apply`。团队负责人创建一个声明式配置文件并提交到团队仓库。新成员只需运行一条命令：

```bash
agk apply https://raw.githubusercontent.com/my-org/configs/main/team.toml --dry-run
agk apply https://raw.githubusercontent.com/my-org/configs/main/team.toml
```

配置文件指定了要挂载的仓库、要激活的 provider 和要创建的 profile。团队中的每个人都会获得一致的配置。

结合上下文切换，适合在多个项目间工作的团队：

```bash
agk context create project-alpha --display-name "Project Alpha"
agk context switch project-alpha
agk apply https://internal.configs/alpha.toml
```

`[Team]`

### 6.2 通过 GitHub 共享仓库

1. 创建一个包含 `skills/` 目录的仓库，按照仓库结构组织。
2. 每个技能是 `skills/` 下的一个文件夹，包含一个 `SKILL.md` 文件。
3. 团队成员将该仓库作为 GitHub 仓库挂载：

```bash
# 在 TUI 中：按 0 → F2 → 输入 "my-org/team-skills"
# 或直接配置：
```

```toml
# ~/.config/agk/config.toml
[team-skills.vault]
type = "github"
repo = "my-org/team-skills"
ref = "main"
path = "skills/"
```

GitHub 仓库使用稀疏检出，因此只会下载指定的子目录，而非整个仓库。

**分支策略：** 使用不同分支对应不同环境（例如，`main` 对应稳定版，`dev` 对应实验版）。更改 `ref` 字段指向所需分支即可。

`[Team]`

### 6.3 分发 Profile

Profile 可以存储在仓库的 `profiles/` 目录中，包含一个 `PROFILE.md` 文件。团队成员可以像安装技能一样从仓库安装 profile。

仓库中的 profile 指定了 provider、技能、MCP 和权限。当团队成员激活 profile 时，AGK 会自动解析依赖关系。

`[Team]`

### 6.4 多项目工作的上下文管理

在多个项目或客户之间切换的团队使用上下文来管理不同的配置：

```bash
# 为每个项目创建上下文
agk context create project-alpha --display-name "Project Alpha"
agk context create project-beta --display-name "Project Beta"

# 切换到某个项目
agk context switch project-alpha

# 每个上下文可以有自己的环境标签
# （local、dev、staging、prod）用于筛选
```

切换上下文时，AGK 会用新上下文的仓库和 provider 替换上一个上下文的配置。这可以防止项目之间的冲突。

`[Team]` `[Org]`

---

## 7. TUI 参考

### 7.1 导航

| 按键 | 操作 |
|---|---|
| `1` | 切换到 Skills 标签页 |
| `2` | 切换到 MCP 标签页 |
| `3` | 切换到 Instructions 标签页 |
| `4` | 切换到 Providers 标签页 |
| `5` | 切换到 Profiles 标签页 |
| `0` | 切换到 Vaults 标签页 |
| `Up` / `Down` | 在列表中上下移动 |
| `Tab` | 切换全局 / 工作区作用域 |
| `Esc`（按两次） | 退出 |
| `Ctrl+C` | 强制退出 |

### 7.2 资源标签页（Skills、Instructions）

| 按键 | 操作 |
|---|---|
| `Space` | 安装 / 卸载 |
| `Enter` | 更新选中的资源 |
| `F5` | 更新所有已安装的资源 |
| `F4` | 从源头刷新仓库 |
| `Ctrl+O` | 在文件管理器中打开资源文件夹 |
| `Ctrl+T` | 在资源文件夹中打开终端 |
| 输入文字 | 筛选 / 搜索（ClawHub 激活时也会搜索 ClawHub） |

### 7.3 MCP 标签页

| 按键 | 操作 |
|---|---|
| `F2` | 注册新 MCP 服务器（5 步向导） |
| `Space` | 为当前作用域启用 / 禁用 MCP 服务器 |
| `Enter` | 测试 MCP 服务器连接 |

### 7.4 Providers 标签页

| 按键 | 操作 |
|---|---|
| `Space` | 激活 / 停用 provider |
| `Enter` | 更新选中的 provider |
| `F4` | 刷新 provider 列表 |

> **警告：** 停用最后一个仍有已安装资源的 provider 时会弹出确认对话框。确认后将从该 provider 的目录中移除已安装的技能文件。

### 7.5 Profiles 标签页

| 按键 | 操作 |
|---|---|
| `F2` | 创建新 profile（向导） |
| `F3` | 编辑选中的 profile |
| `Delete` | 删除选中的 profile（需确认） |

### 7.6 Vaults 标签页

| 按键 | 操作 |
|---|---|
| `F2` | 挂载新仓库（本地路径、GitHub URL 或 ClawHub） |
| `Space` | 切换仓库的激活/停用状态 |
| `F4` | 从源头刷新仓库 |

### 7.7 Profile 向导步骤

Profile 向导会引导你完成以下步骤：

1. **Archetype template** — 从预定义模板中选择或选择 Custom
2. **Profile name** — 除 `/`、`\`、`:` 和 null 外的任意字符；必须唯一
3. **Scope selection** — Workspace 或 Global
4. **Role** — agent 扮演的角色（例如："Senior code reviewer"）
5. **Domain / Specialty** — agent 的专业领域
6. **Collaboration Style** — agent 的沟通方式（例如："Direct and critical"）
7. **Scope Boundaries** — agent 的职责范围之内和之外
8. **Activation Triggers** — agent 何时激活（例如："After any code change"）
9. **Constraints** — agent 必须遵守的规则（例如："Always include a line reference"）
10. **Output Format** — 首选输出格式（例如："Concise bullets, max 5 items"）
11. **Core Responsibilities** — agent 的主要职责
12. **Tool selection** — 按配置的工具白名单
13. **Permission mode** — default、acceptEdits、auto、dontAsk 或 plan
14. **Skill checklist** — 从仓库选择技能（可搜索，显示仓库徽标）
15. **MCP checklist** — 选择 MCP 服务器（显示仓库/已注册徽标）
16. **Review** — 可滚动的 Markdown 预览，显示 token 计数徽标

`[Personal]` `[Team]`

---

## 8. CLI 参考

所有命令支持 `--quiet` / `-q`、`--verbose` / `-v` 和 `--json` 全局标志。

### 退出码

| 代码 | 含义 |
|---|---|
| 0 | 成功 |
| 1 | 一般故障 |
| 2 | 验证失败 |
| 3 | 部分成功 |

### 8.1 核心命令

#### `agk`

启动 TUI。无需任何参数。

#### `agk sync`

将已安装的资源与配置同步（安装缺失的，更新过时的）。

```bash
agk sync [--global] [--dry-run]
```

| 标志 | 说明 |
|---|---|
| `--global` / `-g` | 强制使用全局作用域 |
| `--dry-run` / `-d` | 预览更改但不实际修改 |

#### `agk install <IDENTITY>`

按标识安装指定资源。

```bash
agk install web-browser                 # 按名称
agk install my-vault/web-browser        # 从指定仓库
agk install web-browser:1.2.0           # 指定版本
```

| 标志 | 说明 |
|---|---|
| `--scope <scope>` / `-s` | 目标作用域（`global` 或 `workspace`） |
| `--dry-run` / `-d` | 预览更改但不实际修改 |
| `--provider <provider>` / `-p` | 限制到指定 provider |
| `--evals` | 包含 `evals/` 子目录 |

#### `agk validate`

对照源仓库验证已安装的资源。

```bash
agk validate [--scope <scope>]
```

| 标志 | 说明 |
|---|---|
| `--scope <scope>` / `-s` | 目标作用域（`global` 或 `workspace`） |

#### `agk pack <IDENTITY>`

将技能打包为特定 provider 的可分发格式。

```bash
agk pack web-browser --target claude-desktop
agk pack web-browser --target tarball --stdout > my-skill.tar.gz
```

| 标志 | 说明 |
|---|---|
| `--target <target>` / `-t` | 打包格式：`claude-desktop`、`firebender` 或 `tarball` |
| `--stdout` | 输出到 stdout 而非文件 |

#### `agk clean`

移除 AGK 配置文件。

```bash
agk clean [--global]
```

| 标志 | 说明 |
|---|---|
| `--global` / `-g` | 移除全局配置而非工作区配置 |

### 8.2 上下文命令

#### `agk context switch <NAME>`

切换到指定上下文并应用其默认设置。

```bash
agk context switch acme-corp [--dry-run]
```

#### `agk context list`

列出所有已配置的上下文。

#### `agk context create <NAME>`

创建新上下文。

```bash
agk context create client-x --display-name "Client X"
```

| 标志 | 说明 |
|---|---|
| `--display-name <name>` / `-d` | 人类可读的显示名称 |

### 8.3 Apply 命令

#### `agk apply <SOURCE>`

从 URL 或本地路径应用声明式配置。

```bash
agk apply https://example.com/team.toml
agk apply ./team-config.toml --dry-run
agk apply ./team.toml --context acme-corp --environment prod
```

| 标志 | 说明 |
|---|---|
| `--scope <scope>` / `-s` | 目标作用域（默认：`workspace`） |
| `--context <name>` / `-c` | 目标上下文 |
| `--environment <env>` / `-e` | 目标环境：`local`、`dev`、`staging`、`prod` |
| `--dry-run` | 预览更改但不实际修改 |

### 8.4 MCP 命令

#### `agk mcp add`

注册新 MCP 服务器。

```bash
agk mcp add \
  --name my-server \
  --command "npx" \
  --args "-y,@modelcontextprotocol/server-filesystem,/tmp" \
  --transport stdio \
  --description "Filesystem access"
```

| 标志 | 说明 |
|---|---|
| `--name <name>` / `-n` | 服务器名称（必需，唯一） |
| `--command <cmd>` / `-c` | 要运行的命令（必需） |
| `--args <args>` / `-a` | 参数（逗号分隔） |
| `--env <env>` / `-e` | 环境变量（`KEY=VALUE`，逗号分隔） |
| `--transport <type>` / `-t` | 传输类型：`stdio`（默认）或 `sse` |
| `--description <desc>` / `-d` | 描述 |
| `--no-test` | 注册后跳过连接测试 |

#### `agk mcp enable <NAME>`

为 provider 启用 MCP 服务器。

```bash
agk mcp enable my-server --provider claude-code [--scope global]
```

#### `agk mcp disable <NAME>`

为 provider 禁用 MCP 服务器。

```bash
agk mcp disable my-server --provider claude-code [--scope global]
```

#### `agk mcp list`

列出所有已注册的 MCP 服务器。

```bash
agk mcp list [--provider <provider>]
```

#### `agk mcp test <NAME>`

测试 MCP 服务器连接。

```bash
agk mcp test my-server
```

### 8.5 Profile 命令

> **提示：** `agk profile` 有简写别名 `agk p`——例如，`agk p start my-reviewer`。

#### `agk profile start <NAME>`

启动（运行）一个 profile 会话。

```bash
agk profile start my-reviewer [--dry-run]
```

#### `agk profile create <NAME>`

创建新 profile（无 TUI 向导，纯命令行）。

```bash
agk profile create my-reviewer \
  --provider claude-code \
  --skills "code-reviewer,security-audit" \
  --mcps "my-server" \
  --description "Code review profile" \
  --scope workspace
```

| 标志 | 说明 |
|---|---|
| `--provider <provider>` / `-p` | 使用的 provider（默认：`opencode`） |
| `--skills <list>` / `-s` | 逗号分隔的技能名称 |
| `--mcps <list>` / `-m` | 逗号分隔的 MCP 服务器名称 |
| `--description <desc>` / `-d` | 代理描述（或 markdown 文件路径） |
| `--description-file <path>` | 从 markdown 文件读取描述 |
| `--scope <scope>` | 作用域：`global` 或 `workspace`（默认：`workspace`） |
| `--dry-run` | 预览更改但不实际修改 |

### 8.6 遥测命令

#### `agk telemetry enable`

启用本地遥测收集。

#### `agk telemetry disable`

禁用本地遥测收集。

#### `agk telemetry status`

显示遥测状态（启用/禁用）。

#### `agk telemetry export`

导出遥测数据。

```bash
agk telemetry export                       # JSON 输出到 stdout
agk telemetry export --format csv          # CSV 输出到 stdout
agk telemetry export --output ~/data.json  # 写入文件
```

| 标志 | 说明 |
|---|---|
| `--format <fmt>` | 输出格式：`json`（默认）或 `csv` |
| `--output <path>` | 写入文件（默认：stdout） |

### 8.7 调试命令（隐藏）

这些命令不会在帮助输出中显示。

#### `agk debug tasks`

列出活跃和最近的追踪任务。

#### `agk debug hangs`

检测卡住的任务（运行超过 30 秒）。

#### `agk debug trace`

转储当前 trace span 树（需要 `observability` feature）。

`[Personal]` `[Team]` `[Org]`

---

## 9. 配置参考

### 9.1 全局配置（`~/.config/agk/config.toml`）

```toml
version = 1

# 活跃的仓库 ID（必须与下方仓库段键名匹配）
vaults = ["my-vault", "team-skills"]

# 激活的 provider（通过 TUI 或 CLI 切换）
providers = ["claude-code", "opencode"]

# Provider 根目录覆盖（每个 provider 在工作区中使用哪个文件夹）
[provider_roots]
claude-code = ".claude"     # 选项：".claude"、".agents"
opencode = ".opencode"     # 选项：".opencode"、".agents"
gemini-cli = ".gemini"     # 选项：".gemini"、".ai"

# 仓库定义
[my-vault.vault]
type = "local"
path = "/path/to/my-vault"

[team-skills.vault]
type = "github"
repo = "my-org/team-skills"
ref = "main"
path = "skills/"

# 每个仓库的已安装资源（由 AGK 管理，请勿手动编辑）
[my-vault.skills]
items = ["[web-browser:1.2.0:a13c9ef042]"]

[my-vault.instructions]
items = ["[code-style:--:9ac00ff113]"]

# Profile
[[profiles]]
name = "my-reviewer"
provider_id = "claude-code"
scope = "workspace"
skills = ["code-reviewer", "security-audit"]
mcps = ["my-server"]
permission_mode = "default"
```

### 9.2 工作区配置（`.agk/config.toml`）

工作区配置结构与全局配置相同，但作用域限于当前项目目录。它继承全局配置的仓库和 provider，并添加工作区专属的已安装资源。

```toml
version = 1
vaults = []
providers = ["claude-code"]

[my-vault.skills]
items = ["[web-browser:1.2.0:a13c9ef042]"]
```

### 9.3 MCP 注册表（`~/.config/agk/mcp.toml`）

```toml
[servers.my-server]
name = "my-server"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
transport = "stdio"
description = "Filesystem access server"
tested = true
tested_at = "2024-01-15T10:30:00Z"

[servers.my-server.env]
API_KEY = "secret-value"

[servers.my-server.activation.claude-code]
global = true
workspace = true

# SSE 传输示例
[servers.remote-api]
name = "remote-api"
command = ""
transport = "sse"
url = "https://api.example.com/mcp"
```

### 9.4 上下文（`~/.config/agk/contexts.toml`）

上下文存储在单个 TOML 文件中。当前上下文由 `current_context` 字段跟踪。

```toml
# ~/.config/agk/contexts.toml
current_context = "default"

[contexts.default]
display_name = "Personal"
vaults = ["my-vault"]
providers = ["claude-code"]
profiles = ["my-reviewer"]

[contexts.acme-corp]
display_name = "Acme Corp"
vaults = ["team-skills"]
providers = ["claude-code", "opencode"]
profiles = ["acme-reviewer"]
environment = "prod"

[contexts.acme-corp.tags]
team = "backend"
cost-center = "eng-001"
```

### 9.5 遥测（`~/.config/agk/analytics.toml`）

遥测是可选的，且仅在本地存储。它记录每个 provider 的技能调用及时间戳。

```bash
agk telemetry status    # 检查状态
agk telemetry enable    # 开始收集
agk telemetry export    # 导出为 JSON 或 CSV
```

### 9.6 仓库结构

仓库目录遵循以下结构：

```
my-vault/
  skills/
    web-browser/
      SKILL.md           # 必需——描述该技能
      scripts/            # 可选——可执行脚本
      references/         # 可选——参考文档
      assets/             # 可选——附加文件
      evals/              # 可选——测试用例（使用 --evals 安装）
  instructions/
    code-style/
      AGENTS.md          # 必需——行为提示词
  mcps/
    my-server/
      MCP.md             # 必需——MCP 服务器定义
  profiles/
    reviewer/
      PROFILE.md         # 必需——profile 定义
```

### 9.7 SKILL.md 前置数据

```yaml
---
name: web-browser
version: "1.2.0"
author: "Jane Developer"
description: "Browse the web from your AI agent"
requires:
  - clawhub/http-client
  - clawhub/html-parser
requires_optional:
  - clawhub/cache
---
```

| 字段 | 类型 | 必需 | 说明 |
|---|---|---|---|
| `name` | string | 是 | 技能标识符 |
| `version` | string | 是 | 语义化版本号 |
| `author` | string | 否 | 作者名称 |
| `description` | string | 否 | 简短描述 |
| `requires` | list | 否 | 始终安装的依赖 |
| `requires_optional` | list | 否 | 可以跳过的依赖 |

### 9.8 config.toml 中的 Profile 配置

```toml
[[profiles]]
name = "my-reviewer"
provider_id = "claude-code"
scope = "workspace"

# Skills 可以是纯字符串或带仓库引用的表
skills = ["code-reviewer", { name = "security-audit", vault = "clawhub" }]

# Instructions 格式相同
instructions = ["code-style", { name = "security-rules", vault = "team-skills" }]

# MCPs 格式相同
mcps = ["my-server"]

# 工具限制（provider 专属）
tool_refs = ["Read", "Glob", "Grep"]

# 权限模式："default"、"auto"、"acceptEdits" 或 "plan"
permission_mode = "default"

# 可选：指向 Markdown 文件的路径，用于叠加额外的 prompt 内容
# prompt_overlay_path = "./my-overlay.md"
```

`[Personal]` `[Team]` `[Org]`

---

## 10. 故障排除

### 10.1 安装问题

| 问题 | 解决方案 |
|---|---|
| `cargo install agk` 失败 | 确保安装了 Rust 1.70+。运行 `rustup update` 更新。 |
| 找不到 `agk` 命令 | 将 Cargo 的 bin 目录添加到 `PATH`：`export PATH="$HOME/.cargo/bin:$PATH"` |
| 找不到 Homebrew tap | 确保已添加 tap：`brew tap agk/tap` |

### 10.2 仓库问题

| 问题 | 解决方案 |
|---|---|
| GitHub 仓库克隆失败 | 检查网络连接。确保仓库可访问。尝试 `git ls-remote owner/repo` 验证访问权限。 |
| 找不到本地仓库 | 确认路径是绝对路径或相对于当前目录的正确路径。使用 `pwd` 检查当前目录。 |
| 找不到 ClawHub CLI | 使用 `brew install clawhub` 安装或从 [clawhub.ai](https://clawhub.ai) 下载。如果 Homebrew 不可用，请使用手动下载链接。 |
| F4 刷新卡住 | GitHub 仓库使用稀疏检出。大型仓库可能需要时间。检查网络连接。 |
| 仓库显示没有技能 | 确保仓库结构正确：`skills/<name>/SKILL.md`。检查仓库配置中的 `path` 设置——它应指向包含技能的文件夹，而非仓库根目录。 |

### 10.3 技能和安装问题

| 问题 | 解决方案 |
|---|---|
| "No provider configured" 错误 | 先激活一个 provider（TUI：按 `4`，然后在某个 provider 上按 `Space`）。 |
| 挂载仓库后找不到技能 | 按 `F4` 刷新仓库。检查仓库路径和结构。 |
| 更新后 SHA10 不匹配 | 这意味着技能内容发生了变化。在资源上按 `Enter` 或按 `F5` 更新所有内容。 |
| 元技能依赖解析失败 | 检查技能 `SKILL.md` 中的 `requires:` 列表。确保所有引用的仓库已挂载。 |
| 循环依赖错误 | 某个技能通过依赖链依赖了自身。从 `requires:` 列表中移除循环引用。 |
| 技能文件未出现在 provider 中 | 检查 provider 是否已激活（TUI：Providers 标签页）。检查作用域（全局 vs 工作区——按 `Tab`）。 |

### 10.4 MCP 服务器问题

| 问题 | 解决方案 |
|---|---|
| 握手测试失败 | 验证命令是否正确，以及服务器二进制文件是否在 `PATH` 中。尝试手动运行该命令。 |
| 注册时出现"Command not found" | 确保 MCP 服务器的命令是绝对路径或在系统 `PATH` 中。 |
| SSE 服务器连接失败 | 验证 URL 是否正确以及服务器是否正在运行。检查防火墙或代理问题。 |
| MCP 已启用但 provider 未检测到 | 检查作用域——MCP 服务器可以按作用域启用。使用 `agk mcp list` 验证激活状态。 |

### 10.5 Profile 问题

| 问题 | 解决方案 |
|---|---|
| Profile 向导没有出现 | 只有支持 profile 的 provider（Claude Code、OpenCode）才会显示向导。请先激活支持 profile 的 provider。 |
| "Profile already exists" 错误 | Profile 名称在同一作用域内必须唯一。请使用不同名称或使用 `--scope global`。 |
| 启动 profile 时缺少技能 | Profile 引用了未安装的技能。AGK 会发出警告但不会阻止。请安装引用的技能或从 profile 中移除它们。 |
| "Provider not active or does not support profiles" | 请先激活 provider（TUI：Providers 标签页，按 `Space` 切换）。 |

### 10.6 上下文问题

| 问题 | 解决方案 |
|---|---|
| 切换上下文后未激活预期的仓库 | 检查 `~/.config/agk/contexts.toml` 中该上下文的 `vaults` 列表。确保仓库名称与你的配置匹配。 |
| "Context does not exist" 错误 | 请先创建上下文：`agk context create <name>`。使用 `agk context list` 列出现有上下文。 |

### 10.7 TUI 问题

| 问题 | 解决方案 |
|---|---|
| 终端渲染异常 | 确保你的终端支持真彩色且至少为 80×24。尝试 `export TERM=xterm-256color`。 |
| 快捷键无响应 | 某些终端会拦截功能键。尝试使用其他终端（iTerm2、Kitty、Alacritty）。 |
| 作用域切换不工作 | 按 `Tab` 在全局和工作区作用域之间切换。当前作用域显示在底部状态栏中。 |
| 搜索找不到远程技能 | 确保 ClawHub 仓库已激活（Vaults 标签页，在 `clawhub` 上按 `Space`）。 |

### 10.8 CLI 退出码

| 代码 | 含义 |
|---|---|
| 0 | 成功 |
| 1 | 一般故障 |
| 2 | 验证失败 |
| 3 | 部分成功（部分操作成功，部分失败） |

`[Personal]` `[Team]`

---

## 11. Provider 专属指南

### 11.1 Claude Code

**Provider ID：** `claude-code`

**安装路径（工作区）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `{workspace}/.claude/skills/{name}/`（或 `{workspace}/{provider_root}/skills/{name}/`） |
| Instructions | `{workspace}/.claude/instructions/{name}/` |
| MCP 配置 | `{workspace}/.claude/mcp.json` |
| Profiles | `{workspace}/.claude/agents/{name}.md` |

**安装路径（全局）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `~/.claude/skills/{name}/` |
| Instructions | `~/.claude/instructions/{name}/` |
| MCP 配置 | `~/.claude/mcp.json` |

**Config roots：** `.claude`（默认）或 `.agents`（共享 agents 文件夹）

**Profile 启动：** AGK 生成一个 `agents/{name}.md` 文件（包含 YAML 前置数据：名称、provider、工具、permission_mode、skills、mcps），然后运行 `claude --agent <path>`。

**功能：** Skills ✓ · Instructions ✓ · MCP ✓ · Profiles ✓

### 11.2 OpenCode

**Provider ID：** `opencode`

**安装路径（工作区）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `{workspace}/.opencode/skills/{name}/`（或 `{workspace}/{provider_root}/skills/{name}/`） |
| Instructions | `{workspace}/.opencode/instructions/{name}/` |
| MCP 配置 | `{workspace}/opencode.json`（注意：位于工作区根目录，而非 `.opencode/` 内） |
| Profiles | `{workspace}/.agk/profiles/{name}/agents/{name}.md` |

**安装路径（全局）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `~/.config/opencode/skills/{name}/` |
| Instructions | `~/.config/opencode/instructions/{name}/` |
| MCP 配置 | `~/.config/opencode/opencode.json` |

**Config roots：** `.opencode`（默认）或 `.agents`（与 Claude 兼容）

**Profile 启动：** AGK 将每个 agent 的条目写入 `opencode.json`，运行 `opencode --agent <name>`，并在退出时清理会话 agent 条目。

**功能：** Skills ✓ · Instructions ✓ · MCP ✓ · Profiles ✓

### 11.3 GitHub Copilot

**Provider ID：** `github-copilot`

**安装路径（工作区）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `{workspace}/.github/skills/{name}/` |
| Instructions | `{workspace}/.github/instructions/{name}/` |

**安装路径（全局）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `~/.copilot/skills/{name}/` |
| Instructions | `~/.copilot/instructions/{name}/` |
| MCP 配置 | `~/.copilot/mcp-config.json` |

> **注意：** GitHub Copilot 不支持工作区作用域的 MCP 配置。MCP 仅支持全局作用域。

**功能：** Skills ✓ · Instructions ✓ · MCP ✓（仅全局） · Profiles ✗

### 11.4 Gemini CLI

**Provider ID：** `gemini-cli`

**安装路径（工作区）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `{workspace}/.gemini/skills/{name}/`（或 `{workspace}/{provider_root}/skills/{name}/`） |
| Instructions | `{workspace}/.gemini/instructions/{name}/` |

**安装路径（全局）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `~/.gemini/skills/{name}/` |
| Instructions | `~/.gemini/instructions/{name}/` |
| MCP 配置 | `~/.gemini/settings.json` |

**Config roots：** `.gemini`（默认）或 `.ai`（旧版）

> **注意：** Gemini CLI 不支持工作区作用域的 MCP 配置。MCP 仅支持全局作用域。

**功能：** Skills ✓ · Instructions ✓ · MCP ✓（仅全局） · Profiles ✗

### 11.5 AMP Code

**Provider ID：** `amp`

**安装路径（工作区）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `{workspace}/.amp/skills/{name}/` |
| Instructions | `{workspace}/.amp/instructions/{name}/` |
| MCP 配置 | `{workspace}/.amp/settings.json` |

**安装路径（全局）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `~/.amp/skills/{name}/` |
| Instructions | `~/.amp/instructions/{name}/` |
| MCP 配置 | `~/.config/amp/settings.json` |

> **注意：** AMP 的 MCP 条目嵌套在 `amp.mcpServers` 下（非顶层）。

**功能：** Skills ✓ · Instructions ✓ · MCP ✓ · Profiles ✗

### 11.6 Firebender

**Provider ID：** `firebender`

**安装路径（工作区）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `{workspace}/.firebender/skills/{name}/` |
| Instructions | `{workspace}/.firebender/instructions/{name}/` |

**安装路径（全局）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `~/.firebender/skills/{name}/` |
| Instructions | `~/.firebender/instructions/{name}/` |

**功能：** Skills ✓ · Instructions ✓ · MCP ✗ · Profiles ✗

### 11.7 Letta

**Provider ID：** `letta`

**安装路径（工作区）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `{workspace}/.letta/skills/{name}/` |
| Instructions | `{workspace}/.letta/instructions/{name}/` |

**安装路径（全局）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `~/.letta/skills/{name}/` |
| Instructions | `~/.letta/instructions/{name}/` |

**功能：** Skills ✓ · Instructions ✓ · MCP ✗ · Profiles ✗

### 11.8 Snowflake Cortex

**Provider ID：** `snowflake`

**安装路径（工作区）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `{workspace}/.cortex/skills/{name}/` |
| Instructions | `{workspace}/.cortex/instructions/{name}/` |

**安装路径（全局）：**

| 资源类型 | 路径 |
|---|---|
| Skills | `~/.cortex/skills/{name}/` |
| Instructions | `~/.cortex/instructions/{name}/` |

> **注意：** 文件夹名是 `.cortex`，而非 `.snowflake`。

**功能：** Skills ✓ · Instructions ✓ · MCP ✗ · Profiles ✗

`[Personal]` `[Team]`

---

*AGK 支持指南 — 版本 0.2.x*