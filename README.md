## 📌 目录

- [概述](#概述)
- [核心特性](#核心特性)
- [环境要求](#环境要求)
- [安装步骤](#安装步骤)
- [命令详解](#命令详解)
  - [`srm del` — 安全删除](#srm-del--安全删除)
  - [`srm res` — 恢复文件](#srm-res--恢复文件)
  - [`srm ls` — 列出回收站](#srm-ls--列出回收站)
  - [`srm cln` — 清理过期文件](#srm-cln--清理过期文件)
  - [`srm empty` — 清空回收站](#srm-empty--清空回收站)
- [配置说明](#配置说明)
- [性能优化](#性能优化)
- [日志与审计](#日志与审计)
- [安全替代原生 `rm`](#安全替代原生-rm)
- [许可证](#许可证)

---

## 概述

`srm`（Safe Remove）是一个 **安全替代系统 `rm` 命令** 的工具。它不会真正删除文件，而是将文件移动到用户专属的回收站目录（`~/.srm/trash`），并记录元数据，支持后续恢复、清理和审计。

**核心理念**：  
> “删除” ≠ “永久丢失”，而是“暂时移入安全区”。

---

## 核心特性

✅ **安全删除**：移动而非删除，避免误操作  
✅ **完整恢复**：支持恢复到原始路径或自定义路径  
✅ **中断保护**：`Ctrl+C` 中断后自动回滚  
✅ **系统保护**：默认阻止删除 `/bin`、`/etc` 等关键目录  
✅ **自动清理**：支持自定义过期时间（默认 7 天）  
✅ **结构化日志**：JSONL 格式，保留 30 天  
✅ **高性能**：优先使用 `rename()`（零拷贝）  
✅ **清晰帮助**：每个命令含详细用法和示例

---

## 环境要求

- **操作系统**：Debian 13+ / Ubuntu 22.04+ / macOS
- **架构**：x86_64 / aarch64
- **Rust 版本**：1.70+
- **依赖工具**：`build-essential`, `cargo`, `curl`

---

## 安装步骤

```bash
# 1. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. 克隆并构建
git clone https://github.com/your-username/srm.git
cd srm
RUSTFLAGS="-C target-cpu=native" cargo build --release

# 3. 安装
sudo cp target/release/srm /usr/local/bin/
```

> 可选：使用 `upx --best --lzma target/release/srm` 压缩二进制

---

## 命令详解

### `srm del` — 安全删除

将文件/目录移动到回收站，不真正删除。

#### 用法
```bash
srm del [OPTIONS] <PATHS>...
```

#### 参数
| 参数 | 简写 | 说明 | 默认值 |
|------|------|------|--------|
| `<PATHS>...` | - | 要删除的路径（支持多个） | 必填 |
| `--expire-days <DAYS>` | `-d` | 设置过期天数 | `7` |
| `--force` | `-f` | 强制删除受保护路径 | 无 |

#### 示例
```bash
# 删除单个文件（7天过期）
srm del report.pdf

# 批量删除，设置30天过期
srm del -d 30 logs/ temp/

# 强制删除受保护路径（谨慎！）
srm del -f /tmp/system-test
```

> 🔒 **保护路径**：`/bin`, `/sbin`, `/etc`, `/usr`, `/lib`, `/lib64`, `/root`, `/boot`

---

### `srm res` — 恢复文件

从回收站恢复文件到原始路径或自定义路径。

#### 用法
```bash
srm res [OPTIONS] <NAMES>...
```

#### 参数
| 参数 | 简写 | 说明 |
|------|------|------|
| `<NAMES>...` | - | 回收站文件名（从 `srm ls` 获取） |
| `--force` | `-f` | 强制覆盖目标路径（跳过确认） |
| `--target <PATH>` | `-t` | 恢复到自定义路径 |

#### 示例
```bash
# 列出回收站获取垃圾名
srm ls

# 恢复到原始路径（交互式确认）
srm res file.txt_1735689000000000000

# 恢复到自定义目录
srm res docs_1735... -t ~/recovered/

# 恢复并重命名
srm res file.txt_1735... -t ~/backup/new_name.txt

# 强制覆盖（无提示）
srm res -f test.txt_1735...
```

> 💡 若目标存在，默认提示 `[y/N]`；使用 `-f` 跳过提示。

---

### `srm ls` — 列出回收站

显示回收站中所有文件及其状态。

#### 用法
```bash
srm ls [OPTIONS]
```

#### 参数
| 参数 | 说明 |
|------|------|
| `--expired` | 仅显示已过期项 |

#### 示例
```bash
# 列出所有回收项
srm ls

# 仅列出已过期文件
srm ls --expired
```

#### 输出示例
```
file.txt_1735689000000000000: /home/user/file.txt (Active)
docs_1735689100000000000: /home/user/docs (Expired)
```

---

### `srm cln` — 清理过期文件

删除回收站中已过期的文件。

#### 用法
```bash
srm cln [OPTIONS]
```

#### 参数
| 参数 | 简写 | 说明 |
|------|------|------|
| `--all` | `-a` | 清理所有文件（忽略过期状态） |

#### 示例
```bash
# 清理过期文件（默认行为）
srm cln

# 清空整个回收站（危险！）
srm cln -a
```

> ⚠️ `srm cln -a` 不可逆，请谨慎使用。

---

### `srm empty` — 清空回收站

永久删除回收站中**所有**文件（不可恢复）。

#### 用法
```bash
srm empty [OPTIONS]
```

#### 参数
| 参数 | 简写 | 说明 |
|------|------|------|
| `--yes` | `-y` | 跳过确认提示 |

#### 示例
```bash
# 交互式确认后清空
srm empty

# 直接清空（无提示）
srm empty -y
```

> 🔒 默认要求用户确认，防止误操作。

---

## 配置说明

### 核心配置（代码内置常量）

| 常量 | 默认值 | 说明 | 修改方式 |
|------|--------|------|----------|
| `DEFAULT_EXPIRE_DAYS` | `7` | 默认过期天数 | 编辑 `src/main.rs` |
| `MAX_LOG_AGE_DAYS` | `30` | 日志保留天数 | 编辑 `src/main.rs` |

### 数据存储路径

- **回收站目录**：`~/.srm/trash/`
- **元数据目录**：`~/.srm/meta/`（每个文件一个 `.meta`）
- **日志文件**：`~/.srm/srm.log`（JSONL 格式）

### 修改配置步骤

```bash
# 1. 编辑源码
vim src/main.rs

# 2. 修改常量（例如）
const DEFAULT_EXPIRE_DAYS: i64 = 14;

# 3. 重新编译安装
RUSTFLAGS="-C target-cpu=native" cargo build --release
sudo cp target/release/srm /usr/local/bin/
```

---

## 性能优化

### 编译优化
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

- 启用本机 CPU 指令集（AVX2, SSE4.2 等）
- Rust O3 级别优化
- 可选 UPX 压缩（体积减小 50%+）

### 性能对比（Debian 13 x86_64）
| 操作 | `srm`（优化） | 原生 `rm` |
|------|--------------|----------|
| 删除 1KB 文件 | ~0.08ms | ~0.05ms |
| 删除 1000 文件目录 | ~70ms | ~50ms |

> 💡 性能损耗可接受，换来安全恢复能力。

---

## 日志与审计

### 日志格式（JSONL）
```json
{
  "timestamp": "2026-01-26 18:30:45.123",
  "level": "INFO",
  "message": "Successfully moved to trash",
  "details": {
    "original_path": "/home/user/report.pdf",
    "trash_name": "report.pdf_1735689045123456789"
  }
}
```

### 日志管理
- **位置**：`~/.srm/srm.log`
- **保留**：自动清理 >30 天的日志
- **分析**：
  ```bash
  # 查看删除操作
  jq -r 'select(.message | contains("moved"))' ~/.srm/srm.log

  # 查看错误
  jq -r 'select(.level=="ERROR")' ~/.srm/srm.log
  ```

---

## 安全替代原生 `rm`

> ⚠️ **不建议直接替换系统 `rm`**，推荐以下安全方式：

### 方式一：创建别名（推荐）
在 `~/.bashrc` 或 `~/.zshrc` 中添加：
```bash
alias rm='srm del'
```
然后执行：
```bash
source ~/.bashrc
```

### 方式二：显式使用（最安全）
日常操作中**显式调用 `srm del`**，避免混淆：
```bash
srm del important_file.txt  # 安全删除
command rm temp_file.txt    # 真正删除（绕过 srm）
```

> ✅ **优势**：系统脚本仍使用原生 `rm`，不影响系统稳定性。
