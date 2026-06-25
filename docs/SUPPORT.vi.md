# Hướng dẫn Hỗ trợ AGK

> **AGK** (Agent Kit) là công cụ terminal quản lý skill, instruction, và cấu hình MCP server cho các AI coding assistant — từ một nơi, đến tất cả.

Các badge `[Personal]` `[Team]` `[Org]` cho biết mỗi mục hướng tới đối tượng người dùng nào. Mọi người dùng đều có thể sử dụng mọi tính năng; badge chỉ nhấn mạnh đối tượng chính.

---

## Mục lục

1. [Tổng quan](#1-tổng-quan)
2. [Cài đặt](#2-cài-đặt)
3. [Các khái niệm](#3-các-khái-niệm)
4. [Bắt đầu nhanh](#4-bắt-đầu-nhanh)
5. [Hướng dẫn từng bước](#5-hướng-dẫn-từng-bước)
6. [Hướng dẫn cho Team & Tổ chức](#6-hướng-dẫn-cho-team--tổ-chức)
7. [Tham khảo TUI](#7-tham-khảo-tui)
8. [Tham khảo CLI](#8-tham-khảo-cli)
9. [Tham khảo Cấu hình](#9-tham-khảo-cấu-hình)
10. [Khắc phục sự cố](#10-khắc-phục-sự-cố)
11. [Hướng dẫn theo Provider](#11-hướng-dẫn-theo-provider)

---

## 1. Tổng quan

Bạn sử dụng nhiều công cụ AI coding — Claude Code, GitHub Copilot, Gemini CLI, OpenCode, và các công cụ khác. Mỗi công cụ có cấu trúc thư mục riêng, định dạng cấu hình riêng, cách thêm skill và instruction riêng. Việc giữ tất cả chúng đồng bộ rất thủ công và dễ sai.

**AGK giải quyết vấn đề này.** Hãy tưởng tượng AGK là một trung tâm điều khiển phát cấu hình AI agent của bạn đến mọi provider cùng lúc:

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

**AGK làm được gì:**
- **Skills** — Công cụ tái sử dụng giúp AI agent của bạn thông minh hơn (giống như cài app trên điện thoại)
- **Instructions** — Quy tắc hành vi định hình cách AI agent phản hồi (giống như tùy chỉnh hệ thống)
- **MCP Servers** — Cầu nối kết nối AI của bạn với các dịch vụ bên ngoài (giống như tiện ích mở rộng trình duyệt)
- **Profiles** — Các cấu hình được đặt tên, gói gọn provider, skill, MCP, và quyền hạn (giống như màn hình chủ cho từng mục đích sử dụng — riêng cho công việc, riêng cho cá nhân)
- **Vaults** — Nguồn cung cấp skill, instruction, và profile (giống như cửa hàng ứng dụng)
- **Contexts** — Không gian làm việc có thể chuyển đổi cho các team hoặc dự án khác nhau (giống như tài khoản người dùng trên máy tính)

**Các provider được hỗ trợ:** Claude Code · OpenCode · GitHub Copilot · Gemini CLI · AMP Code · Firebender · Letta · Snowflake Cortex

`[Personal]` `[Team]` `[Org]`

---

## 2. Cài đặt

### Homebrew (macOS và Linux)

```bash
brew tap agk/tap
brew install agk
```

### Cargo (build từ mã nguồn)

```bash
cargo install agk
```

Yêu cầu Rust toolchain phiên bản mới. Cài đặt Rust từ [rustup.rs](https://rustup.rs/) nếu cần.

### Binary có sẵn

Tải từ [GitHub Releases](https://github.com/agk-cli/agk/releases) và thêm vào `PATH`.

### Xác minh cài đặt

```bash
agk --version
```

Nếu bạn thấy số phiên bản, bạn đã sẵn sàng.

`[Personal]` `[Team]`

---

## 3. Các khái niệm

### 3.1 Skills

Một **skill** là công cụ mà AI agent của bạn có thể sử dụng. Nó là một thư mục chứa file `SKILL.md` và các thư mục con tùy chọn (`scripts/`, `references/`, `assets/`).

Hãy nghĩ về skill như một **ứng dụng** bạn cài trên điện thoại — nó thêm một khả năng mới.

```
my-vault/
  skills/
    web-browser/
      SKILL.md          # Bắt buộc — mô tả skill
      scripts/           # Tùy chọn — các script thực thi
      references/        # Tùy chọn — tài liệu tham khảo
      assets/            # Tùy chọn — các file bổ sung
```

Khi bạn cài đặt một skill, AGK sao chép nó vào thư mục skill của provider (ví dụ, `~/.claude/skills/web-browser/` cho Claude Code).

`[Personal]` `[Team]`

### 3.2 Instructions

Một **instruction** là quy tắc hành vi cho AI agent của bạn. Nó là một thư mục chứa file `AGENTS.md`.

Hãy nghĩ về instruction như một **thiết lập hệ thống** — nó định hình hành vi thay vì thêm công cụ. Ví dụ, một instruction có thể yêu cầu "luôn viết test trước" hoặc "trả lời bằng bullet points."

Instructions được cài vào các thư mục riêng của từng provider (ví dụ, `.claude/instructions/my-rule/` cho Claude Code ở phạm vi workspace).

`[Personal]` `[Team]`

### 3.3 MCP Servers

Một **MCP server** (Model Context Protocol) là cầu nối giữa AI agent của bạn và một dịch vụ bên ngoài — cơ sở dữ liệu, hệ thống file, trình duyệt, API.

Hãy nghĩ về MCP server như một **tiện ích mở rộng trình duyệt** — nó cắm vào và cung cấp cho AI những khả năng mới mà trước đó không có.

MCP server có thể sử dụng hai loại giao thức truyền tải:
- **stdio** — Server chạy như một tiến trình cục bộ. AGK khởi chạy và giao tiếp qua đầu vào/đầu ra chuẩn.
- **SSE** — Server chạy như một dịch vụ HTTP từ xa. AGK kết nối đến một URL.

Khi bạn đăng ký một MCP server, AGK lưu trữ nó trong `~/.config/agk/mcp.toml` và có thể kích hoạt cho từng provider, từng phạm vi.

> **Mẹo:** Sau khi đăng ký MCP server, AGK tự động chạy thử nghiệm handshake để xác minh kết nối. Badge `[✓]` nghĩa là thử nghiệm đã thành công.

`[Personal]` `[Team]`

### 3.4 Providers

Một **provider** là nền tảng AI nơi skill và instruction của bạn được cài đặt. Hãy nghĩ về nó như **điện thoại** — AGK cài các ứng dụng của bạn lên bất kỳ điện thoại nào bạn chọn.

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

Một số provider (Claude Code, OpenCode, Gemini CLI) cho phép bạn chọn **config root** — tên thư mục nơi skill và instruction được lưu trữ. Ví dụ, OpenCode có thể dùng `.opencode` (mặc định) hoặc `.agents` (tương thích với Claude). Bạn chọn điều này lần đầu khi kích hoạt provider.

`[Personal]` `[Team]`

### 3.5 Vaults

Một **vault** là nơi skill đến từ. Hãy nghĩ về nó như một **cửa hàng ứng dụng** — bạn gắn vault, duyệt nội dung bên trong, và cài những gì bạn cần.

AGK hỗ trợ ba loại vault:

| Loại vault | Cách hoạt động | Ví dụ |
|---|---|---|
| **Local** | Một thư mục trên ổ đĩa của bạn | `./my-vault` |
| **GitHub** | Một repository GitHub (sparse checkout) | `owner/repo` |
| **ClawHub** | Thị trường cộng đồng ClawHub | Tích hợp sẵn, nhấn để kích hoạt |

Đối với vault GitHub, AGK sử dụng `git sparse-checkout` để chỉ tải thư mục con bạn cần, giúp mọi thứ nhanh chóng. Bạn chỉ định nhánh (mặc định: `main`) và đường dẫn thư mục con (mặc định: `skills/`).

Đối với ClawHub, AGK sử dụng CLI `clawhub` để tìm kiếm và cài đặt các gói cộng đồng. Nếu CLI chưa được cài, AGK đề xuất cài qua Homebrew hoặc cung cấp liên kết tải xuống thủ công.

`[Personal]` `[Team]` `[Org]`

### 3.6 Profiles

Một **profile** là một cấu hình được đặt tên, hoàn chỉnh, gói gọn provider cùng các skill, MCP, instruction và cài đặt quyền hạn đã chọn. Hãy nghĩ về nó như **bố cục màn hình chủ** — cùng điện thoại, nhưng sắp xếp ứng dụng khác nhau cho công việc so với sử dụng cá nhân.

Khi bạn khởi động một profile, AGK:
1. Tạo file markdown agent với các công cụ và quyền hạn đã chọn
2. Cập nhật cấu hình của provider với các MCP server và quyền hạn skill
3. Khởi chạy CLI của provider
4. Dọn dẹp mọi thứ khi phiên kết thúc

Profile được tạo qua wizard TUI (nhấn `F2` ở tab Profiles) hoặc CLI (`agk profile create`).

**Các archetype của profile wizard:**

| Archetype | Vai trò | Công cụ mặc định | Chế độ quyền |
|---|---|---|---|
| Code Reviewer | Reviewer cấp cao | Read, Glob, Grep, LSP | default |
| Feature Implementer | Kỹ sư cấp cao | Read, Glob, Grep, Bash, Write, Edit | default |
| Security Auditor | Kỹ sư bảo mật | Read, Glob, Grep, Bash | default |
| Documentation Writer | Kỹ sư tài liệu | Read, Glob, Grep, Write, Edit | default |
| Test Generator | Kỹ sư QA | Read, Glob, Grep, Bash, Write | default |
| Custom | Trang trắng | — | — |

**Các chế độ quyền:**

| Chế độ | Hành vi |
|---|---|
| `default` | Xác nhận trước khi chỉnh sửa |
| `acceptEdits` | Tự động chấp nhận chỉnh sửa |
| `auto` | Tự động phê duyệt thao tác an toàn |
| `dontAsk` | Không bao giờ yêu cầu xác nhận |
| `plan` | Chế độ kế hoạch — chỉ đề xuất, không thực thi |

`[Personal]` `[Team]`

### 3.7 Contexts

Một **context** là không gian làm việc có thể chuyển đổi và được đặt tên. Hãy nghĩ về nó như một **tài khoản người dùng** trên máy tính — một cái cho dự án cá nhân, một cái cho công ty, một cái cho khách hàng cụ thể.

Mỗi context mang theo:
- Tên hiển thị (ví dụ, "Personal", "Acme Corp", "Client X")
- Danh sách vault
- Danh sách provider
- Danh sách profile
- Nhãn môi trường (local, dev, staging, prod)
- Các tag (cặp khóa-giá trị)

Context mặc định có tên `default` với tên hiển thị "Personal". Khi chuyển context, AGK gộp các vault và provider của context đó vào cấu hình đang hoạt động.

```bash
agk context list               # Hiển thị tất cả context
agk context switch acme-corp   # Chuyển sang context acme-corp
agk context create client-x --display-name "Client X"  # Tạo context mới
```

`[Team]` `[Org]`

### 3.8 Phạm vi (Global vs Workspace)

**Phạm vi** xác định nơi AGK lưu trữ cấu hình và asset đã cài.

| Phạm vi | Đường dẫn cấu hình | Nội dung |
|---|---|---|
| **Global** | `~/.config/agk/config.toml` | Định nghĩa vault, kích hoạt provider, profile toàn cục |
| **Workspace** | `.agk/config.toml` | Asset đã cài cho dự án này |

Hãy nghĩ về nó như **Cài đặt Hệ thống so với Cài đặt Ứng dụng** — phạm vi global áp dụng ở khắp nơi trên máy, phạm vi workspace chỉ áp dụng trong một thư mục dự án.

Trong TUI, nhấn `Tab` để chuyển đổi giữa các phạm vi. Trong CLI, sử dụng `--scope global` hoặc `--scope workspace`.

> **Lưu ý:** Vault và provider thường được cấu hình ở phạm vi global. Asset đã cài (skill, instruction) thường được theo dõi ở phạm vi workspace.

`[Personal]` `[Team]`

### 3.9 Phát hiện Thay đổi SHA10

AGK theo dõi xem asset đã cài của bạn có cập nhật hay không bằng **SHA10** — dấu vân tay nội dung của mỗi asset. Nó băm file `SKILL.md` của skill cùng các thư mục `scripts/`, `references/`, và `assets/`, rồi lấy 10 ký tự đầu tiên.

Một asset hiển thị là **cập nhật** khi SHA10 đã cài khớp với SHA10 đã quét. Nếu ai đó cập nhật skill trong vault, SHA10 thay đổi ngay cả khi số phiên bản không đổi, vì vậy AGK luôn biết khi nào bạn cần cập nhật.

Trong TUI, nhấn `Enter` trên một asset lỗi thời để cập nhật, hoặc `F5` để cập nhật tất cả cùng lúc.

`[Personal]` `[Team]`

### 3.10 Meta-skills và Phụ thuộc

Một **meta-skill** là skill có `SKILL.md` frontmatter liệt kê các skill khác làm phụ thuộc. Hãy nghĩ về nó như một **gói tổng hợp** — cài nó là cài tất cả những gì nó cần.

```yaml
# SKILL.md frontmatter
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

- `requires` — Phụ thuộc luôn được cài.
- `requires_optional` — Phụ thuộc mà người dùng có thể chọn bỏ qua.

AGK phân giải phụ thuộc đệ quy. Nếu hai meta-skill phụ thuộc vào cùng một skill, nó chỉ được cài một lần (loại bỏ trùng lặp kiểu diamond). Phụ thuộc tuần hoàn được phát hiện và từ chối với thông báo lỗi.

`[Team]`

---

## 4. Bắt đầu nhanh

Làm theo phần này để đi từ con số 0 đến thiết lập hoạt động trong chưa tới 5 phút.

### 4.1 Khởi động TUI

```bash
agk
```

Bạn sẽ thấy giao diện terminal toàn màn hình với các tab ở trên và danh sách phím tắt ở dưới.

### 4.2 Gắn vault

1. Nhấn `0` để chuyển sang tab **Vaults**.
2. Nhấn `F2` để gắn vault mới.
3. Nhập đường dẫn cục bộ (ví dụ, `./my-vault`) hoặc URL GitHub (ví dụ, `my-org/team-skills`).
4. Đối với vault GitHub: xác nhận nhánh (mặc định `main`) và đường dẫn thư mục con (mặc định `skills/`).
5. Nhập tên cho vault (mặc định là tên thư mục hoặc repo).

Ngoài ra, kích hoạt vault ClawHub tích hợp bằng cách nhấn `Space` trên mục `clawhub` trong tab Vaults.

### 4.3 Kích hoạt provider

1. Nhấn `4` để chuyển sang tab **Providers**.
2. Nhấn `Space` trên provider bạn muốn kích hoạt (ví dụ, `claude-code`).
3. Nếu provider hỗ trợ nhiều config root, chọn một từ hộp thoại.

### 4.4 Cài đặt skill đầu tiên

1. Nhấn `1` để chuyển sang tab **Skills**.
2. Gõ để tìm kiếm skill theo tên.
3. Nhấn `Space` trên skill bạn muốn cài.
4. AGK sao chép file skill vào thư mục skill của provider và ghi lại việc cài đặt trong `config.toml`.

### 4.5 Đăng ký MCP server

1. Nhấn `2` để chuyển sang tab **MCP**.
2. Nhấn `F2` để bắt đầu wizard đăng ký.
3. Điền 5 bước: **Tên**, **Lệnh**, **Tham số**, **Giao thức** (stdio hoặc SSE), **Mô tả**.
4. AGK tự động chạy thử nghiệm handshake. Nếu thành công, bạn sẽ thấy badge `[✓]`.

### 4.6 Tạo profile

1. Nhấn `5` để chuyển sang tab **Profiles**.
2. Nhấn `F2` để bắt đầu profile wizard.
3. Làm theo các bước: tên, phạm vi, mẫu archetype, câu hỏi định danh, danh sách skill, danh sách MCP, chọn công cụ/quyền, xem lại.
4. Để khởi động profile:

```bash
agk profile start my-profile
```

`[Personal]`

---

## 5. Hướng dẫn từng bước

### 5.1 Quản lý Vaults

**Gắn vault cục bộ:**

```bash
# Qua TUI: Nhấn 0 → F2 → nhập đường dẫn → nhập tên
# Hoặc qua file cấu hình:
```

```toml
# ~/.config/agk/config.toml
[my-vault.vault]
type = "local"
path = "/path/to/my-vault"
```

**Gắn vault GitHub:**

```bash
# Qua TUI: Nhấn 0 → F2 → nhập "owner/repo" → xác nhận nhánh → xác nhận đường dẫn → nhập tên
# Hoặc qua file cấu hình:
```

```toml
[team-skills.vault]
type = "github"
repo = "my-org/team-skills"
ref = "main"
path = "skills/"
```

**Kích hoạt ClawHub:**
- Trong TUI, chuyển đến tab Vaults và nhấn `Space` trên mục `clawhub`.
- Nếu CLI `clawhub` chưa được cài, AGK đề xuất cài qua Homebrew hoặc cung cấp liên kết tải xuống thủ công.

**Gỡ vault:**
- Trong TUI, chuyển đến tab Vaults, chọn vault, và nhấn `Space` để tắt. Hộp thoại xác nhận sẽ xuất hiện.

**Làm mới vault:**
- Nhấn `F4` trong bất kỳ tab nào để làm mới tất cả vault từ nguồn.

`[Personal]` `[Team]`

### 5.2 Cài đặt và Cập nhật Skills

**Cài đặt một skill:**

```bash
# TUI: Nhấn 1 → gõ để tìm kiếm → Space để cài
# CLI:
agk install web-browser
agk install my-vault/web-browser       # từ vault cụ thể
agk install web-browser:1.2.0          # phiên bản cụ thể
```

**Cập nhật một skill:**
- Trong TUI, chọn skill và nhấn `Enter`.

**Cập nhật tất cả skill:**
- Nhấn `F5` trong bất kỳ tab nào.

**Bao gồm evals khi cài:**

```bash
agk install web-browser --evals
```

Flag `--evals` bao gồm thư mục con `evals/` (các test case) trong quá trình cài đặt.

`[Personal]` `[Team]`

### 5.3 Làm việc với Instructions

Instructions hoạt động theo cùng cơ chế với skill — `Space` để cài, `Enter` để cập nhật, `F5` để cập nhật hàng loạt. Điểm khác biệt là nội dung: `AGENTS.md` chứa các prompt hành vi thay vì các định nghĩa công cụ `SKILL.md`.

Xem [Mục 11](#11-hướng-dẫn-theo-provider) để biết instruction được cài ở đâu cho từng provider.

`[Personal]` `[Team]`

### 5.4 Quản lý MCP Server

**Đăng ký MCP server:**

```bash
# TUI: Nhấn 2 → F2 → điền 5 bước
# CLI:
agk mcp add \
  --name my-server \
  --command "npx" \
  --args "-y,@modelcontextprotocol/server-filesystem,/tmp" \
  --transport stdio \
  --description "Filesystem access server"
```

**Kích hoạt MCP server cho một provider:**

```bash
agk mcp enable my-server --provider claude-code
agk mcp enable my-server --provider claude-code --scope global
```

**Vô hiệu hóa MCP server:**

```bash
agk mcp disable my-server --provider claude-code
```

**Liệt kê các MCP server đã đăng ký:**

```bash
agk mcp list
agk mcp list --provider claude-code
```

**Kiểm tra kết nối MCP server:**

```bash
agk mcp test my-server
```

> **Cảnh báo:** Thử nghiệm handshake MCP chạy lệnh server trên máy của bạn. Chỉ đăng ký các MCP server mà bạn tin tưởng.

> **Lưu ý:** Đối với giao thức SSE, hãy truyền URL server qua `--url` (ví dụ `agk mcp add --name remote --transport sse --url https://mcp.example.com/sse`). Nếu bỏ qua `--url` khi dùng `--transport sse`, giá trị mặc định là `http://localhost:3000`.

`[Personal]` `[Team]`

### 5.5 Tạo và Khởi động Profiles

**Tạo qua TUI wizard (khuyến nghị):**
- Nhấn `5` → `F2` → làm theo wizard nhiều bước.

**Tạo qua CLI:**

```bash
agk profile create my-reviewer \
  --provider claude-code \
  --skills "code-reviewer,security-audit" \
  --mcps "my-server" \
  --description "Reviews code for quality and security" \
  --scope workspace
```

**Khởi động profile:**

```bash
agk profile start my-reviewer
```

**Xem trước mà không chạy:**

```bash
agk profile start my-reviewer --dry-run
```

Lệnh này hiển thị kế hoạch khởi động (file nào sẽ được tạo, cấu hình nào sẽ được cập nhật) mà không thực sự bắt đầu phiên.

`[Personal]` `[Team]`

### 5.6 Chuyển đổi Contexts

```bash
# Liệt kê tất cả context
agk context list

# Chuyển sang một context
agk context switch acme-corp

# Tạo context mới
agk context create client-x --display-name "Client X"
```

Khi bạn chuyển context, AGK gộp các vault và provider của context mới vào cấu hình global đang hoạt động. Các thay đổi từ context trước sẽ được gỡ bỏ trước.

Context được lưu trữ trong `~/.config/agk/contexts.toml`.

`[Team]` `[Org]`

### 5.7 Áp dụng Cấu hình Khai báo (Onboarding Team)

`agk apply` đọc một file `team.toml` cục bộ và đồng bộ thiết lập cục bộ của bạn cho khớp với nó. Hãy nghĩ về nó như `docker compose up` cho công cụ AI — bạn mô tả những gì bạn muốn, và `apply` biến nó thành hiện thực.

```bash
# Áp dụng từ file cục bộ
agk apply ./team-config.toml

# Xem trước mà không thay đổi
agk apply ./team-config.toml --dry-run

# Áp dụng cho context và môi trường cụ thể
agk apply ./team-config.toml --context acme-corp --environment prod
```

> **Lưu ý:** Nguồn URL (`http://` / `https://`) chưa được hỗ trợ — `agk apply` chỉ phân giải đường dẫn file cục bộ. Để áp dụng cấu hình từ xa, hãy tải về trước (ví dụ `curl -o team.toml https://...`) rồi truyền đường dẫn cục bộ.

Nguồn cấu hình có thể chỉ định vault, provider, profile, và MCP server. `agk apply` thêm các mục còn thiếu, cập nhật các mục đã thay đổi, và gỡ bỏ các mục không còn trong nguồn.

`[Team]` `[Org]`

### 5.8 Đồng bộ Assets

```bash
# Đồng bộ tất cả asset đã cấu hình (cài thiếu, cập nhật cũ)
agk sync

# Đồng bộ ở phạm vi global
agk sync --global

# Xem trước mà không thay đổi
agk sync --dry-run
```

`[Personal]` `[Team]`

### 5.9 Đóng gói Skills để Phân phối

```bash
# Đóng gói cho Claude Desktop
agk pack web-browser --target claude-desktop

# Đóng gói thành tarball
agk pack web-browser --target tarball

# Xuất ra stdout (có thể pipe)
agk pack web-browser --target tarball --stdout > my-skill.tar.gz
```

Các mục tiêu đóng gói: `claude-desktop`, `firebender`, `tarball`.

`[Team]`

### 5.10 Telemetry và Thông tin Sử dụng

AGK chỉ thu thập telemetry ở mức cục bộ — không có dữ liệu nào được gửi ra ngoài. Dữ liệu được lưu trong `~/.config/agk/analytics.toml`.

```bash
agk telemetry status            # Kiểm tra xem telemetry đã bật chưa
agk telemetry enable            # Bật telemetry
agk telemetry disable           # Tắt telemetry
agk telemetry export            # Xuất dưới dạng JSON (mặc định)
agk telemetry export --format csv   # Xuất dưới dạng CSV
agk telemetry export --output ~/analytics.json  # Ghi ra file
```

`[Personal]` `[Team]`

### 5.11 Dọn dẹp

```bash
# Gỡ cấu hình workspace
agk clean

# Gỡ cấu hình global
agk clean --global
```

> **Cảnh báo:** `agk clean` gỡ các file cấu hình. File skill đã cài trong thư mục provider không bị gỡ — chỉ cấu hình AGK bị xóa.

`[Personal]`

---

## 6. Hướng dẫn cho Team & Tổ chức

### 6.1 Onboarding Team với Apply

Cách nhanh nhất để thiết lập cho thành viên team mới là dùng `agk apply`. Team lead tạo file cấu hình khai báo và commit vào repository của team. Thành viên mới checkout file đó về máy rồi chạy một lệnh trên file cục bộ:

```bash
agk apply ./team.toml --dry-run
agk apply ./team.toml
```

File cấu hình chỉ định vault cần gắn, provider cần kích hoạt, và profile cần tạo. Mọi người trong team sẽ có cùng một thiết lập.

Kết hợp với chuyển đổi context cho các team làm việc trên nhiều dự án:

```bash
agk context create project-alpha --display-name "Project Alpha"
agk context switch project-alpha
agk apply ./alpha.toml
```

`[Team]`

### 6.2 Chia sẻ Vaults qua GitHub

1. Tạo repository với thư mục `skills/` theo cấu trúc vault.
2. Mỗi skill là một thư mục dưới `skills/` với file `SKILL.md`.
3. Thành viên team gắn repo như một vault GitHub:

```bash
# Trong TUI: Nhấn 0 → F2 → nhập "my-org/team-skills"
# Hoặc cấu hình trực tiếp:
```

```toml
# ~/.config/agk/config.toml
[team-skills.vault]
type = "github"
repo = "my-org/team-skills"
ref = "main"
path = "skills/"
```

Vault GitHub sử dụng sparse checkout, vì vậy chỉ thư mục con được chỉ định được tải — không phải toàn bộ repository.

**Chiến lược nhánh:** Sử dụng các nhánh khác nhau cho các môi trường khác nhau (ví dụ, `main` cho ổn định, `dev` cho thử nghiệm). Thay đổi trường `ref` để trỏ đến nhánh mong muốn.

`[Team]`

### 6.3 Phân phối Profiles

Profile có thể được lưu trữ trong vault dưới thư mục `profiles/` với file `PROFILE.md`. Thành viên team cài profile từ vault giống như cài skill.

Một profile trong vault chỉ định provider, skill, MCP, và quyền hạn. Khi thành viên team kích hoạt profile, AGK tự động phân giải các phụ thuộc.

`[Team]`

### 6.4 Quản lý Context cho Công việc Đa Dự án

Các team làm việc trên nhiều dự án hoặc khách hàng sử dụng context để chuyển đổi giữa các cấu hình khác nhau:

```bash
# Tạo context cho mỗi dự án
agk context create project-alpha --display-name "Project Alpha"
agk context create project-beta --display-name "Project Beta"

# Chuyển sang một dự án
agk context switch project-alpha

# Mỗi context có thể có nhãn môi trường riêng
# (local, dev, staging, prod) để lọc
```

Khi bạn chuyển context, AGK thay thế vault và provider của context trước bằng vault và provider của context mới. Điều này ngăn xung đột giữa các dự án.

`[Team]` `[Org]`

---

## 7. Tham khảo TUI

### 7.1 Điều hướng

| Phím | Hành động |
|---|---|
| `1` | Chuyển sang tab Skills |
| `2` | Chuyển sang tab MCP |
| `3` | Chuyển sang tab Instructions |
| `4` | Chuyển sang tab Providers |
| `5` | Chuyển sang tab Profiles |
| `0` | Chuyển sang tab Vaults |
| `Up` / `Down` | Di chuyển trong danh sách |
| `Tab` | Chuyển đổi phạm vi Global / Workspace |
| `Esc` (hai lần) | Thoát |
| `Ctrl+C` | Buộc thoát |

### 7.2 Tab Asset (Skills, Instructions)

| Phím | Hành động |
|---|---|
| `Space` | Cài / Gỡ cài |
| `Enter` | Cập nhật asset đã chọn |
| `F5` | Cập nhật tất cả asset đã cài |
| `F4` | Làm mới vault từ nguồn |
| `Ctrl+O` | Mở thư mục asset trong trình quản lý file |
| `Ctrl+T` | Mở terminal tại thư mục asset |
| Gõ | Lọc / tìm kiếm (cũng tìm trên ClawHub khi đang kích hoạt) |

### 7.3 Tab MCP

| Phím | Hành động |
|---|---|
| `F2` | Đăng ký MCP server mới (wizard 5 bước) |
| `Space` | Kích hoạt / Vô hiệu hóa MCP server cho phạm vi hiện tại |
| `Enter` | Kiểm tra kết nối MCP server |

### 7.4 Tab Providers

| Phím | Hành động |
|---|---|
| `Space` | Kích hoạt / Vô hiệu hóa provider |
| `F4` | Làm mới danh sách provider |
| `Enter` | Cập nhật provider đang chọn |

> **Cảnh báo:** Vô hiệu hóa provider cuối cùng có asset đã cài sẽ hiển thị hộp thoại xác nhận. Xác nhận sẽ gỡ file skill đã cài từ thư mục của provider đó.

### 7.5 Tab Profiles

| Phím | Hành động |
|---|---|
| `F2` | Tạo profile mới (wizard) |
| `F3` | Chỉnh sửa profile đang chọn |
| `Delete` | Xóa profile đã chọn (kèm xác nhận) |

### 7.6 Tab Vaults

| Phím | Hành động |
|---|---|
| `F2` | Gắn vault mới (đường dẫn cục bộ, URL GitHub, hoặc ClawHub) |
| `Space` | Bật/tắt trạng thái active của vault |
| `F4` | Làm mới vault từ nguồn |

### 7.7 Các bước Profile Wizard

Profile wizard hướng dẫn qua các bước sau:

1. **Archetype template** — chọn từ template có sẵn hoặc Custom
2. **Profile name** — bất kỳ ký tự nào trừ `/`, `\`, `:`, và null; phải là duy nhất
3. **Scope selection** — Workspace hoặc Global
4. **Role** — vai trò của agent (ví dụ: "Senior code reviewer")
5. **Domain / Specialty** — lĩnh vực chuyên môn của agent
6. **Collaboration Style** — cách agent giao tiếp (ví dụ: "Direct and critical")
7. **Scope Boundaries** — những gì nằm trong và ngoài phạm vi của agent
8. **Activation Triggers** — khi nào agent nên kích hoạt (ví dụ: "After any code change")
9. **Constraints** — quy tắc agent phải tuân theo (ví dụ: "Always include a line reference")
10. **Output Format** — định dạng đầu ra ưu tiên (ví dụ: "Concise bullets, max 5 items")
11. **Core Responsibilities** — nhiệm vụ chính của agent
12. **Tool selection** — danh sách công cụ theo provider
13. **Permission mode** — default, acceptEdits, auto, dontAsk, hoặc plan
14. **Skill checklist** — chọn skill từ vault (có thể tìm kiếm, hiển thị badge vault)
15. **MCP checklist** — chọn MCP server (hiển thị badge vault/đã đăng ký)
16. **Review** — xem trước markdown có thể cuộn, hiển thị badge số token

`[Personal]` `[Team]`

---

## 8. Tham khảo CLI

Tất cả lệnh hỗ trợ các flag toàn cục `--quiet` / `-q`, `--verbose` / `-v`, và `--json`.

### Mã thoát

| Mã | Ý nghĩa |
|---|---|
| 0 | Thành công |
| 1 | Lỗi chung |
| 2 | Lỗi xác thực |
| 3 | Thành công một phần |

### 8.1 Các lệnh cốt lõi

#### `agk`

Khởi động TUI. Không cần tham số.

#### `agk sync`

Đồng bộ asset đã cài với cấu hình (cài những mục còn thiếu, cập nhật những mục lỗi thời).

```bash
agk sync [--global] [--dry-run]
```

| Flag | Mô tả |
|---|---|
| `--global` / `-g` | Buộc phạm vi global |
| `--dry-run` / `-d` | Xem trước thay đổi mà không chỉnh sửa |

#### `agk install <IDENTITY>`

Cài một asset cụ thể theo định danh.

```bash
agk install web-browser                 # theo tên
agk install my-vault/web-browser        # từ vault cụ thể
agk install web-browser:1.2.0           # phiên bản cụ thể
```

| Flag | Mô tả |
|---|---|
| `--scope <scope>` / `-s` | Phạm vi đích (`global` hoặc `workspace`) |
| `--dry-run` / `-d` | Xem trước thay đổi mà không chỉnh sửa |
| `--provider <provider>` / `-p` | Giới hạn cho provider cụ thể |
| `--evals` | Bao gồm thư mục con `evals/` |

#### `agk validate`

Xác thực asset đã cài với vault nguồn.

```bash
agk validate [--scope <scope>]
```

| Flag | Mô tả |
|---|---|
| `--scope <scope>` / `-s` | Phạm vi mục tiêu (`global` hoặc `workspace`) |

#### `agk pack <IDENTITY>`

Đóng gói skill thành dạng phân phối theo provider.

```bash
agk pack web-browser --target claude-desktop
agk pack web-browser --target tarball --stdout > my-skill.tar.gz
```

| Flag | Mô tả |
|---|---|
| `--target <target>` / `-t` | Định dạng đóng gói: `claude-desktop`, `firebender`, hoặc `tarball` |
| `--stdout` | Ghi ra stdout thay vì file |

#### `agk clean`

Gỡ các file cấu hình AGK.

```bash
agk clean [--global]
```

| Flag | Mô tả |
|---|---|
| `--global` / `-g` | Gỡ cấu hình global thay vì cấu hình workspace |

### 8.2 Các lệnh Context

#### `agk context switch <NAME>`

Chuyển sang một context và áp dụng các cài đặt mặc định của nó.

```bash
agk context switch acme-corp [--dry-run]
```

#### `agk context list`

Liệt kê tất cả context đã cấu hình.

#### `agk context create <NAME>`

Tạo context mới.

```bash
agk context create client-x --display-name "Client X"
```

| Flag | Mô tả |
|---|---|
| `--display-name <name>` / `-d` | Tên hiển thị cho người đọc |

### 8.3 Lệnh Apply

#### `agk apply <SOURCE>`

Áp dụng cấu hình khai báo từ một file `team.toml` cục bộ.

```bash
agk apply ./team-config.toml --dry-run
agk apply ./team.toml --context acme-corp --environment prod
```

> **Lưu ý:** Nguồn URL (`http://` / `https://`) chưa được hỗ trợ; hãy truyền đường dẫn file cục bộ.

| Flag | Mô tả |
|---|---|
| `--scope <scope>` / `-s` | Phạm vi đích (mặc định: `workspace`) |
| `--context <name>` / `-c` | Context đích |
| `--environment <env>` / `-e` | Môi trường đích: `local`, `dev`, `staging`, `prod` |
| `--dry-run` | Xem trước thay đổi mà không chỉnh sửa |

### 8.4 Các lệnh MCP

#### `agk mcp add`

Đăng ký MCP server mới.

```bash
agk mcp add \
  --name my-server \
  --command "npx" \
  --args "-y,@modelcontextprotocol/server-filesystem,/tmp" \
  --transport stdio \
  --description "Filesystem access"
```

| Flag | Mô tả |
|---|---|
| `--name <name>` / `-n` | Tên server (bắt buộc, duy nhất) |
| `--command <cmd>` / `-c` | Lệnh chạy (bắt buộc) |
| `--args <args>` / `-a` | Tham số (phân cách bằng dấu phẩy) |
| `--env <env>` / `-e` | Biến môi trường (`KEY=VALUE`, phân cách bằng dấu phẩy) |
| `--transport <type>` / `-t` | Loại giao thức: `stdio` (mặc định) hoặc `sse` |
| `--url <url>` | URL cho giao thức SSE (dùng với `--transport sse`; mặc định `http://localhost:3000` nếu bỏ qua) |
| `--description <desc>` / `-d` | Mô tả |
| `--no-test` | Bỏ qua kiểm tra kết nối sau khi đăng ký |

#### `agk mcp enable <NAME>`

Kích hoạt MCP server cho một provider.

```bash
agk mcp enable my-server --provider claude-code [--scope global]
```

#### `agk mcp disable <NAME>`

Vô hiệu hóa MCP server cho một provider.

```bash
agk mcp disable my-server --provider claude-code [--scope global]
```

#### `agk mcp list`

Liệt kê tất cả MCP server đã đăng ký.

```bash
agk mcp list [--provider <provider>]
```

#### `agk mcp test <NAME>`

Kiểm tra kết nối MCP server.

```bash
agk mcp test my-server
```

### 8.5 Các lệnh Profile

> **Mẹo:** `agk profile` có alias viết tắt `agk p` — ví dụ, `agk p start my-reviewer`.

#### `agk profile list`

Liệt kê tất cả profile đã cấu hình trong scope mục tiêu.

```bash
agk profile list [--scope <scope>]
```

| Flag | Mô tả |
|---|---|
| `--scope <scope>` / `-s` | Scope mục tiêu: `global` hoặc `workspace` (mặc định: `workspace`) |

#### `agk profile start <NAME>`

Khởi động phiên profile.

```bash
agk profile start my-reviewer [--dry-run]
```

#### `agk profile create <NAME>`

Tạo profile mới (không có TUI wizard).

```bash
agk profile create my-reviewer \
  --provider claude-code \
  --skills "code-reviewer,security-audit" \
  --mcps "my-server" \
  --description "Code review profile" \
  --scope workspace
```

| Flag | Mô tả |
|---|---|
| `--provider <provider>` / `-p` | Provider sử dụng (mặc định: `opencode`) |
| `--skills <list>` / `-k` | Tên skill phân cách bằng dấu phẩy |
| `--mcps <list>` / `-m` | Tên MCP server phân cách bằng dấu phẩy |
| `--description <desc>` / `-d` | Mô tả agent (hoặc đường dẫn đến file markdown) |
| `--description-file <path>` | Đọc mô tả từ file markdown |
| `--scope <scope>` / `-s` | Phạm vi: `global` hoặc `workspace` (mặc định: `workspace`) |
| `--dry-run` | Xem trước thay đổi mà không chỉnh sửa |

### 8.6 Các lệnh Telemetry

#### `agk telemetry enable`

Bật thu thập telemetry cục bộ.

#### `agk telemetry disable`

Tắt thu thập telemetry cục bộ.

#### `agk telemetry status`

Hiển thị trạng thái telemetry (bật/tắt).

#### `agk telemetry export`

Xuất dữ liệu telemetry.

```bash
agk telemetry export                       # JSON ra stdout
agk telemetry export --format csv          # CSV ra stdout
agk telemetry export --output ~/data.json  # Ghi ra file
```

| Flag | Mô tả |
|---|---|
| `--format <fmt>` | Định dạng đầu ra: `json` (mặc định) hoặc `csv` |
| `--output <path>` | Ghi ra file (mặc định: stdout) |

### 8.7 Các lệnh Debug (Ẩn)

Các lệnh này không hiển thị trong kết quả help.

#### `agk debug tasks`

Liệt kê các task đang theo dõi hoạt động và gần đây.

#### `agk debug hangs`

Phát hiện task treo (chạy lâu hơn 30 giây).

#### `agk debug trace`

Xuất cây trace span hiện tại (yêu cầu tính năng `observability`).

`[Personal]` `[Team]` `[Org]`

---

## 9. Tham khảo Cấu hình

### 9.1 Cấu hình Global (`~/.config/agk/config.toml`)

```toml
version = 1

# ID vault đang hoạt động (phải khớp với khóa section vault bên dưới)
vaults = ["my-vault", "team-skills"]

# Provider đang kích hoạt (bật/tắt qua TUI hoặc CLI)
providers = ["claude-code", "opencode"]

# Ghi đè root provider (thư mục nào mỗi provider sử dụng trong workspace)
[provider_roots]
claude-code = ".claude"     # Tùy chọn: ".claude", ".agents"
opencode = ".opencode"     # Tùy chọn: ".opencode", ".agents"
gemini-cli = ".gemini"     # Tùy chọn: ".gemini", ".ai"

# Định nghĩa vault
[my-vault.vault]
type = "local"
path = "/path/to/my-vault"

[team-skills.vault]
type = "github"
repo = "my-org/team-skills"
ref = "main"
path = "skills/"

# Asset đã cài theo vault (do AGK quản lý, không tự chỉnh sửa)
[my-vault.skills]
items = ["[web-browser:1.2.0:a13c9ef042]"]

[my-vault.instructions]
items = ["[code-style:--:9ac00ff113]"]

# Profiles
[[profiles]]
name = "my-reviewer"
provider_id = "claude-code"
scope = "workspace"
skills = ["code-reviewer", "security-audit"]
mcps = ["my-server"]
permission_mode = "default"
```

### 9.2 Cấu hình Workspace (`.agk/config.toml`)

Cấu hình workspace có cùng cấu trúc với cấu hình global nhưng được giới hạn trong thư mục dự án hiện tại. Nó kế thừa vault và provider từ cấu hình global và thêm các asset đã cài đặc thù cho workspace.

```toml
version = 1
vaults = []
providers = ["claude-code"]

[my-vault.skills]
items = ["[web-browser:1.2.0:a13c9ef042]"]
```

### 9.3 MCP Registry (`~/.config/agk/mcp.toml`)

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

# Ví dụ giao thức SSE
[servers.remote-api]
name = "remote-api"
command = ""
transport = "sse"
url = "https://api.example.com/mcp"
```

### 9.4 Contexts (`~/.config/agk/contexts.toml`)

Context được lưu trữ trong một file TOML duy nhất. Context hiện tại được theo dõi bởi trường `current_context`.

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

### 9.5 Telemetry (`~/.config/agk/analytics.toml`)

Telemetry là tùy chọn và chỉ được lưu cục bộ. Nó theo dõi các lần gọi skill theo provider cùng mốc thời gian.

```bash
agk telemetry status    # Kiểm tra trạng thái
agk telemetry enable    # Bắt đầu thu thập
agk telemetry export    # Xuất dưới dạng JSON hoặc CSV
```

### 9.6 Cấu trúc Vault

Một thư mục vault tuân theo cấu trúc sau:

```
my-vault/
  skills/
    web-browser/
      SKILL.md           # Bắt buộc — mô tả skill
      scripts/            # Tùy chọn — các script thực thi
      references/         # Tùy chọn — tài liệu tham khảo
      assets/             # Tùy chọn — các file bổ sung
      evals/              # Tùy chọn — test case (cài với --evals)
  instructions/
    code-style/
      AGENTS.md          # Bắt buộc — prompt hành vi
  mcps/
    my-server/
      MCP.md             # Bắt buộc — định nghĩa MCP server
  profiles/
    reviewer/
      PROFILE.md         # Bắt buộc — định nghĩa profile
```

### 9.7 SKILL.md Frontmatter

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

| Trường | Kiểu | Bắt buộc | Mô tả |
|---|---|---|---|
| `name` | string | Có | Định danh skill |
| `version` | string | Có | Phiên bản ngữ nghĩa |
| `author` | string | Không | Tên tác giả |
| `description` | string | Không | Mô tả ngắn |
| `requires` | list | Không | Phụ thuộc luôn được cài |
| `requires_optional` | list | Không | Phụ thuộc có thể bỏ qua |

### 9.8 Cấu hình Profile trong config.toml

```toml
[[profiles]]
name = "my-reviewer"
provider_id = "claude-code"
scope = "workspace"

# Skill có thể là chuỗi đơn giản hoặc bảng với tham chiếu vault
skills = ["code-reviewer", { name = "security-audit", vault = "clawhub" }]

# Instruction theo cùng định dạng
instructions = ["code-style", { name = "security-rules", vault = "team-skills" }]

# MCP theo cùng định dạng
mcps = ["my-server"]

# Giới hạn công cụ (theo provider)
tool_refs = ["Read", "Glob", "Grep"]

# Chế độ quyền: "default", "auto", "acceptEdits", hoặc "plan"
permission_mode = "default"

# Tùy chọn: đường dẫn đến file markdown phủ thêm nội dung prompt
# prompt_overlay_path = "./my-overlay.md"
```

`[Personal]` `[Team]` `[Org]`

---

## 10. Khắc phục sự cố

### 10.1 Vấn đề Cài đặt

| Vấn đề | Giải pháp |
|---|---|
| `cargo install agk` thất bại | Đảm bảo bạn đã cài Rust 1.70+. Chạy `rustup update` để cập nhật. |
| Không tìm thấy lệnh `agk` | Thêm thư mục bin của Cargo vào `PATH`: `export PATH="$HOME/.cargo/bin:$PATH"` |
| Không tìm thấy Homebrew tap | Đảm bảo bạn đã thêm tap: `brew tap agk/tap` |

### 10.2 Vấn đề Vault

| Vấn đề | Giải pháp |
|---|---|
| Clone vault GitHub thất bại | Kiểm tra kết nối mạng. Đảm bảo repo có thể truy cập. Thử `git ls-remote owner/repo` để xác minh. |
| Không tìm thấy vault cục bộ | Xác minh đường dẫn là tuyệt đối hoặc tương đối so với thư mục hiện tại. Dùng `pwd` để kiểm tra. |
| Không tìm thấy ClawHub CLI | Cài bằng `brew install clawhub` hoặc tải từ [clawhub.ai](https://clawhub.ai). Nếu Homebrew không khả dụng, dùng liên kết tải xuống thủ công. |
| F4 làm mới bị treo | Vault GitHub sử dụng sparse checkout. Repo lớn có thể mất thời gian. Kiểm tra kết nối mạng. |
| Vault không hiển thị skill | Đảm bảo vault có đúng cấu trúc: `skills/<name>/SKILL.md`. Kiểm tra cài đặt `path` trong cấu hình vault — nó phải trỏ đến thư mục chứa skill, không phải root của repo. |

### 10.3 Vấn đề Skill và Cài đặt

| Vấn đề | Giải pháp |
|---|---|
| Lỗi "No provider configured" | Kích hoạt provider trước (TUI: nhấn `4`, rồi `Space` trên một provider). |
| Không tìm thấy skill sau khi gắn vault | Nhấn `F4` để làm mới vault. Kiểm tra đường dẫn và cấu trúc vault. |
| SHA10 không khớp sau khi cập nhật | Điều này nghĩa là nội dung skill đã thay đổi. Nhấn `Enter` trên asset hoặc `F5` để cập nhật tất cả. |
| Thất bại khi phân giải phụ thuộc meta-skill | Kiểm tra danh sách `requires:` trong `SKILL.md` của skill. Đảm bảo tất cả vault được tham chiếu đã được gắn. |
| Lỗi phụ thuộc tuần hoàn | Một skill phụ thuộc vào chính nó qua chuỗi. Gỡ tham chiếu tuần hoàn khỏi danh sách `requires:`. |
| File skill không xuất hiện trong provider | Kiểm tra provider đã được kích hoạt (TUI: tab Providers). Kiểm tra phạm vi (Global so với Workspace — nhấn `Tab`). |

### 10.4 Vấn đề MCP Server

| Vấn đề | Giải pháp |
|---|---|
| Thử nghiệm handshake thất bại | Xác minh lệnh chính xác và binary của server nằm trên `PATH`. Thử chạy lệnh thủ công. |
| "Command not found" khi đăng ký | Đảm bảo lệnh MCP server là đường dẫn tuyệt đối hoặc nằm trên `PATH` của hệ thống. |
| Kết nối SSE server thất bại | Xác minh URL chính xác và server đang chạy. Kiểm tra vấn đề tường lửa hoặc proxy. |
| MCP đã kích hoạt nhưng provider không thấy | Kiểm tra phạm vi — MCP server có thể được kích hoạt theo từng phạm vi. Dùng `agk mcp list` để xác minh trạng thái kích hoạt. |

### 10.5 Vấn đề Profile

| Vấn đề | Giải pháp |
|---|---|
| Profile wizard không xuất hiện | Chỉ provider hỗ trợ profile (Claude Code, OpenCode) hiển thị wizard. Kích hoạt provider hỗ trợ profile trước. |
| Lỗi "Profile already exists" | Tên profile phải duy nhất trong phạm vi. Chọn tên khác hoặc dùng `--scope global`. |
| Thiếu skill khi khởi động profile | Profile tham chiếu skill chưa cài. AGK cảnh báo nhưng không chặn. Cài các skill được tham chiếu hoặc gỡ chúng khỏi profile. |
| "Provider not active or does not support profiles" | Kích hoạt provider trước (TUI: tab Providers, `Space` để bật/tắt). |

### 10.6 Vấn đề Context

| Vấn đề | Giải pháp |
|---|---|
| Chuyển context không kích hoạt vault như mong đợi | Kiểm tra danh sách `vaults` của context trong `~/.config/agk/contexts.toml`. Đảm bảo tên vault khớp với cấu hình. |
| Lỗi "Context does not exist" | Tạo context trước: `agk context create <name>`. Liệt kê context hiện có bằng `agk context list`. |

### 10.7 Vấn đề TUI

| Vấn đề | Giải pháp |
|---|---|
| Hiển thị terminal bị lỗi | Đảm bảo terminal hỗ trợ true color và có kích thước ít nhất 80×24. Thử `export TERM=xterm-256color`. |
| Phím tắt không phản hồi | Một số terminal chặn phím chức năng. Thử terminal khác (iTerm2, Kitty, Alacritty). |
| Chuyển đổi phạm vi không hoạt động | Nhấn `Tab` để chuyển đổi giữa phạm vi Global và Workspace. Phạm vi hiện tại hiển thị ở cuối màn hình. |
| Tìm kiếm không thấy skill từ xa | Đảm bảo vault ClawHub đã kích hoạt (tab Vaults, `Space` trên `clawhub`). |

### 10.8 Mã Thoát CLI

| Mã | Ý nghĩa |
|---|---|
| 0 | Thành công |
| 1 | Lỗi chung |
| 2 | Lỗi xác thực |
| 3 | Thành công một phần (một số thao tác thành công, một số thất bại) |

`[Personal]` `[Team]`

---

## 11. Hướng dẫn theo Provider

### 11.1 Claude Code

**Provider ID:** `claude-code`

**Đường dẫn cài (workspace):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `{workspace}/.claude/skills/{name}/` (hoặc `{workspace}/{provider_root}/skills/{name}/`) |
| Instructions | `{workspace}/.claude/instructions/{name}/` |
| MCP config | `{workspace}/.claude/mcp.json` |
| Profiles | `{workspace}/.claude/agents/{name}.md` |

**Đường dẫn cài (global):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `~/.claude/skills/{name}/` |
| Instructions | `~/.claude/instructions/{name}/` |
| MCP config | `~/.claude/mcp.json` |

**Config roots:** `.claude` (mặc định) hoặc `.agents` (thư mục agent dùng chung)

**Khởi động profile:** AGK tạo file `agents/{name}.md` với YAML frontmatter (name, provider, tools, permission_mode, skills, mcps) và chạy `claude --agent <path>`.

**Khả năng:** Skills ✓ · Instructions ✓ · MCP ✓ · Profiles ✓

### 11.2 OpenCode

**Provider ID:** `opencode`

**Đường dẫn cài (workspace):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `{workspace}/.opencode/skills/{name}/` (hoặc `{workspace}/{provider_root}/skills/{name}/`) |
| Instructions | `{workspace}/.opencode/instructions/{name}/` |
| MCP config | `{workspace}/opencode.json` (lưu ý: ở root workspace, không trong `.opencode/`) |
| Profiles | `{workspace}/.agk/profiles/{name}/agents/{name}.md` |

**Đường dẫn cài (global):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `~/.config/opencode/skills/{name}/` |
| Instructions | `~/.config/opencode/instructions/{name}/` |
| MCP config | `~/.config/opencode/opencode.json` |

**Config roots:** `.opencode` (mặc định) hoặc `.agents` (tương thích Claude)

**Khởi động profile:** AGK cập nhật `opencode.json` với các mục per-agent và chạy `opencode --agent <name>`, rồi dọn dẹp mục agent khi thoát.

**Khả năng:** Skills ✓ · Instructions ✓ · MCP ✓ · Profiles ✓

### 11.3 GitHub Copilot

**Provider ID:** `github-copilot`

**Đường dẫn cài (workspace):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `{workspace}/.github/skills/{name}/` |
| Instructions | `{workspace}/.github/instructions/{name}/` |

**Đường dẫn cài (global):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `~/.copilot/skills/{name}/` |
| Instructions | `~/.copilot/instructions/{name}/` |
| MCP config | `~/.copilot/mcp-config.json` |

> **Lưu ý:** GitHub Copilot không hỗ trợ cấu hình MCP ở phạm vi workspace. MCP chỉ hỗ trợ ở phạm vi global.

**Khả năng:** Skills ✓ · Instructions ✓ · MCP ✓ (chỉ global) · Profiles ✗

### 11.4 Gemini CLI

**Provider ID:** `gemini-cli`

**Đường dẫn cài (workspace):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `{workspace}/.gemini/skills/{name}/` (hoặc `{workspace}/{provider_root}/skills/{name}/`) |
| Instructions | `{workspace}/.gemini/instructions/{name}/` |

**Đường dẫn cài (global):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `~/.gemini/skills/{name}/` |
| Instructions | `~/.gemini/instructions/{name}/` |
| MCP config | `~/.gemini/settings.json` |

**Config roots:** `.gemini` (mặc định) hoặc `.ai` (cũ)

> **Lưu ý:** Gemini CLI không hỗ trợ cấu hình MCP ở phạm vi workspace. MCP chỉ hỗ trợ ở phạm vi global.

**Khả năng:** Skills ✓ · Instructions ✓ · MCP ✓ (chỉ global) · Profiles ✗

### 11.5 AMP Code

**Provider ID:** `amp`

**Đường dẫn cài (workspace):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `{workspace}/.amp/skills/{name}/` |
| Instructions | `{workspace}/.amp/instructions/{name}/` |
| MCP config | `{workspace}/.amp/settings.json` |

**Đường dẫn cài (global):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `~/.amp/skills/{name}/` |
| Instructions | `~/.amp/instructions/{name}/` |
| MCP config | `~/.config/amp/settings.json` |

> **Lưu ý:** Các mục MCP của AMP nằm lồng dưới `amp.mcpServers` (không phải ở cấp gốc).

**Khả năng:** Skills ✓ · Instructions ✓ · MCP ✓ · Profiles ✗

### 11.6 Firebender

**Provider ID:** `firebender`

**Đường dẫn cài (workspace):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `{workspace}/.firebender/skills/{name}/` |
| Instructions | `{workspace}/.firebender/instructions/{name}/` |

**Đường dẫn cài (global):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `~/.firebender/skills/{name}/` |
| Instructions | `~/.firebender/instructions/{name}/` |

**Khả năng:** Skills ✓ · Instructions ✓ · MCP ✗ · Profiles ✗

### 11.7 Letta

**Provider ID:** `letta`

**Đường dẫn cài (workspace):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `{workspace}/.letta/skills/{name}/` |
| Instructions | `{workspace}/.letta/instructions/{name}/` |

**Đường dẫn cài (global):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `~/.letta/skills/{name}/` |
| Instructions | `~/.letta/instructions/{name}/` |

**Khả năng:** Skills ✓ · Instructions ✓ · MCP ✗ · Profiles ✗

### 11.8 Snowflake Cortex

**Provider ID:** `snowflake`

**Đường dẫn cài (workspace):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `{workspace}/.cortex/skills/{name}/` |
| Instructions | `{workspace}/.cortex/instructions/{name}/` |

**Đường dẫn cài (global):**

| Loại asset | Đường dẫn |
|---|---|
| Skills | `~/.cortex/skills/{name}/` |
| Instructions | `~/.cortex/instructions/{name}/` |

> **Lưu ý:** Tên thư mục là `.cortex`, không phải `.snowflake`.

**Khả năng:** Skills ✓ · Instructions ✓ · MCP ✗ · Profiles ✗

`[Personal]` `[Team]`

---

*Hướng dẫn Hỗ trợ AGK — phiên bản 0.2.x*