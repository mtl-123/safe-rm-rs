# srm - 安全文件删除工具（带审计追踪与回收站功能）

> 基于Rust开发的`rm`安全替代工具，提供可恢复的文件删除、审计日志、回收站管理、定期自动清理能力，支持按用户独立配置且不影响系统原生`rm`命令，适配TB级文件操作性能

## 目录

- [概述](#概述)
- [核心特性](#核心特性)
- [开发与运行环境](#开发与运行环境)
- [安装步骤](#安装步骤)
- [核心命令详解](#核心命令详解)
- [自动清理与Systemd服务配置](#自动清理与systemd服务配置)
- [安全替代原生rm（按用户独立生效）](#安全替代原生rm按用户独立生效)
- [配置说明](#配置说明)
- [性能优化特性](#性能优化特性)
- [日志与审计](#日志与审计)
- [安全防护机制](#安全防护机制)
- [常见问题](#常见问题)

## 概述

`srm`（Safe RM）是一款面向Linux系统的安全文件删除工具，核心解决原生`rm`命令**删除不可恢复、无操作记录、无安全校验**的痛点。工具将待删除文件/目录移动至专属回收站，生成唯一短ID用于快速恢复，记录全量审计日志，并支持按过期时间自动清理、手动恢复/永久删除，同时针对大文件/大目录做了极致性能优化，适配企业级TB-scale操作场景。

所有数据（回收站、元数据、日志）均存储在`srm`可执行文件同级的`.srm`目录中，实现**按用户/按安装目录隔离**，不修改系统全局配置，完全不影响原生`rm`命令的使用，兼顾安全性与系统兼容性。

## 核心特性

结合源码实现的核心功能，全维度覆盖安全、性能、易用性：

1. **可恢复的删除机制**：删除文件并非直接销毁，而是移动至专属回收站，通过短ID可快速恢复，避免误删损失
2. **唯一短ID标识**：每个删除项生成6位带类型前缀的短ID（文件`f_`/目录`d_`/软链`l_`），支持短ID快速恢复/查询
3. **全量审计日志**：记录所有操作（删除/恢复/清理/空回收站），包含操作人、时间、路径、大小、权限、执行结果等元数据，日志自动轮转（30天保留）
4. **完善的回收站管理**：支持列出回收站（含过期状态/大小/过期时间）、恢复指定项、清理过期项、永久清空回收站
5. **TB级性能优化**：针对大文件/目录做多层优化，支持同文件系统即时重命名、跨文件系统写时复制（CoW）、内存映射I/O（mmap）、分块传输
6. **实时进度追踪**：大文件（>100MB）/大目录（>5项/>100个文件）操作时显示实时进度条，包含耗时、吞吐量、剩余时间
7. **系统路径保护**：默认禁止删除`/bin`/`/etc`/`/usr`等8个核心系统路径，防止误删导致系统崩溃，可通过`-f`强制覆盖
8. **磁盘空间校验**：删除前检查目标文件系统可用空间，防止回收站占满磁盘，单文件最大占用可用空间80%
9. **原子化元数据存储**：删除项的元数据（原路径/回收站路径/权限/UID/GID等）采用原子化写入，防止数据损坏
10. **中断安全与回滚**：支持Ctrl+C中断操作，正在执行的删除任务会自动回滚，避免文件丢失/损坏
11. **权限严格隔离**：回收站、日志、元数据目录/文件均设置`0700/0600`权限，仅当前用户可访问，防止越权查看/修改
12. **跨Linux发行版兼容**：基于Rust跨平台特性，无需修改代码即可在主流Linux发行版运行

## 开发与运行环境

### 开发环境（精准匹配提供的配置）

- 操作系统：Debian 13 Trixie Desktop
- 开发语言：Rust（2021 Edition）
- 构建工具：Cargo 1.93.0 (083ac5135 2025-12-15)
- 开发依赖组件（Debian 13）：
  - build-essential：基础编译工具链
  - libssl-dev：SSL/TSL依赖（日志序列化/解析）
  - pkg-config：系统依赖管理工具
  - rustup：Rust版本管理工具
  - systemd-dev：Systemd服务开发依赖（可选，用于自动清理服务）
  - git：代码版本控制工具（可选）

### 运行环境

- 操作系统：Linux（内核≥3.10，支持`fallocate`/`ioctl FICLONE`）
- 支持的Linux发行版：
  1. Debian 11/12/13（Bullseye/Bookworm/Trixie）
  2. Ubuntu 20.04/22.04/24.04 LTS
  3. CentOS Stream 8/9、RHEL 8/9
  4. Fedora 38/39/40
  5. Arch Linux/Manjaro（滚动更新版）
  6. openSUSE Leap 15.5/Tumbleweed
- 系统依赖：
  - libc6 ≥2.28：系统基础C库
  - systemd ≥240：（可选，用于自动清理服务部署）
- 硬件要求：无特殊要求，磁盘剩余空间≥回收站所需空间（建议≥1GB）

## 安装步骤

### 方式1：源码编译安装（推荐，匹配开发环境）

1. 克隆代码（或解压本地代码包）

   ```bash
   # 若为本地代码，直接进入代码目录即可
   git clone <你的代码仓库地址> srm && cd srm
   ```

2. 安装开发依赖（Debian 13 环境）

   ```bash
   sudo apt update && sudo apt install -y build-essential libssl-dev pkg-config rustup git
   ```

3. 初始化Rust环境（若未安装）

   ```bash
   rustup default stable
   source $HOME/.cargo/env  # 加载Rust环境变量
   ```

4. 编译并安装

   ```bash
   cargo build --release  # 生产环境编译，开启LTO/优化
   ```

5. 压缩二进制文件（全局安装，所有用户可执行）

   ```bash
   upx --best --lzma target/release/srm
   sudo cp target/release/srm /usr/local/bin/ # 复制到系统可执行目录
   sudo chmod +x /usr/local/bin/srm # 添加执行权限
   ```

6. 验证安装成功（自动创建`.srm`核心目录）

   ```bash
   srm --version
   # 输出：srm 1.2.1 (Meitao Lin <mtl>) 即为成功
   ```

### 方式2：本地二进制直接使用

若已编译好`target/release/srm`二进制文件，可直接复制到任意目录并添加执行权限：

```bash
cp target/release/srm /path/to/your/dir/
chmod +x /path/to/your/dir/srm
# 将该目录添加到PATH，即可全局使用
echo 'export PATH=$PATH:/path/to/your/dir' >> $HOME/.bashrc
source $HOME/.bashrc
```

## 核心命令详解

### 基础说明

- 所有子命令支持**别名**（如`delete`/`del`、`restore`/`res`），简化输入
- 回收站核心目录：`srm`可执行文件同级的`.srm/`，包含`trash/`（回收站文件）、`meta/`（元数据）、`srm.log`（审计日志）
- 短ID规则：6位字符，前缀标识类型（`f_`=文件、`d_`=目录、`l_`=软链），如`f_a3b4c5`、`d_789abc`

### 1. 删除文件/目录（核心命令）

将文件/目录移动至回收站，生成短ID，默认7天后自动过期（可自定义）

```bash
# 基础语法
srm delete [选项] <文件/目录路径>...
# 别名：srm del（推荐，更简洁）
```

#### 选项

| 选项            | 简写 | 说明                                    | 默认值 |
| --------------- | ---- | --------------------------------------- | ------ |
| `--expire-days` | `-d` | 自定义过期天数（过期后可自动清理）      | 7天    |
| `--force`       | `-f` | 强制删除：允许删除保护路径/含`..`的路径 | 禁用   |

#### 示例

```bash
# 删除单个文件，默认7天过期
srm del test.txt
# 递归删除目录，自定义15天过期
srm del -d 15 /data/temp_dir/
# 强制删除系统保护路径下的自定义文件（谨慎使用）
srm del -f /usr/local/my_temp_file
```

#### 执行结果

```
✅ test.txt → 🆔 f_a3b4c5 [1.2 MB]
```

- 生成短ID`f_a3b4c5`，后续可通过该ID恢复
- 自动记录元数据和审计日志

### 2. 恢复回收站文件/目录

通过**短ID**或**回收站全ID**恢复指定项，支持覆盖已存在的文件

```bash
# 基础语法
srm restore [选项] <短ID/回收站ID>...
# 别名：srm res（推荐）
```

#### 选项

| 选项       | 简写 | 说明                               |
| ---------- | ---- | ---------------------------------- |
| `--force`  | `-f` | 强制覆盖目标路径已存在的文件/目录  |
| `--target` | `-t` | 自定义恢复路径（默认恢复到原路径） |

#### 示例

```bash
# 通过短ID恢复到原路径
srm res f_a3b4c5
# 恢复多个项，强制覆盖已存在的文件
srm res -f f_a3b4c5 d_789abc
# 恢复到自定义路径
srm res -t /home/user/restore_dir f_a3b4c5
```

#### 执行结果

```
✅ Restored: f_a3b4c5 → /home/user/test.txt
```

### 3. 列出回收站内容

查看回收站中所有项的状态（短ID/原路径/大小/过期时间/是否过期），支持详细模式

```bash
# 基础语法
srm list [选项]
# 别名：srm ls（推荐）
```

#### 选项

| 选项        | 简写 | 说明                                                |
| ----------- | ---- | --------------------------------------------------- |
| `--expired` | -    | 仅显示已过期的项                                    |
| `--verbose` | `-v` | 详细模式：显示全量元数据（权限/UID/GID/删除时间等） |

#### 示例

```bash
# 简易列出（默认）
srm ls
# 仅显示已过期的项
srm ls --expired
# 详细模式列出所有项
srm ls -v
```

#### 简易模式执行结果

```
📦 Active items (2):
🆔 SHORT      ORIGINAL PATH                             EXPIRES IN    SIZE
------------ ----------------------------------------- ------------ ---------------
f_a3b4c5     /home/user/test.txt                       6d 12h        1.2 MB
d_789abc     /home/user/temp_dir                       14d 5h        890 MB (dir)

🗑️  Expired items (1):
🆔 SHORT      ORIGINAL PATH                             EXPIRED       SIZE
------------ ----------------------------------------- ------------ ---------------
l_xyz123     /home/user/link_to_data                   2d 3h ago     0 B (symlink)

💡 Use `srm ls -v` for detailed view, `srm ls --expired` for expired items
```

### 4. 清理回收站

清理回收站中**已过期的项**（默认）或**所有项**，释放磁盘空间

```bash
# 基础语法
srm clean [选项]
# 别名：srm cln（推荐）
```

#### 选项

| 选项    | 简写 | 说明                                 |
| ------- | ---- | ------------------------------------ |
| `--all` | `-a` | 清理所有项（无论是否过期），谨慎使用 |

#### 示例

```bash
# 清理已过期的项（默认，推荐）
srm cln
# 清理回收站所有项（永久删除，不可恢复）
srm cln -a
```

#### 执行结果

```
🗑️  Cleaned: l_xyz123 (/home/user/link_to_data)

✅ Clean completed! 1 item(s) removed (0 B total)
```

### 5. 永久清空回收站

删除回收站中**所有项**及元数据，**操作不可恢复**，需确认

```bash
# 基础语法
srm empty [选项]
# 别名：srm empty（无简写）
```

#### 选项

| 选项    | 简写 | 说明                   |
| ------- | ---- | ---------------------- |
| `--yes` | `-y` | 跳过确认提示，直接清空 |

#### 示例

```bash
# 清空回收站（需手动确认）
srm empty
# 跳过确认，直接清空（谨慎使用）
srm empty -y
```

#### 执行结果

```
⚠️  Empty trash permanently? This cannot be undone! [y/N]: y
✅ Trash emptied! 3 item(s) permanently deleted (891.2 MB total)
```

### 全局帮助

查看所有命令和选项说明：

```bash
srm --help
# 查看具体子命令帮助
srm del --help
srm res --help
```

## 自动清理与Systemd服务配置

针对**定期清理过期回收站数据**的需求，可将`srm clean`配置为Systemd服务，实现开机自启、定时执行，步骤如下：

### 步骤1：创建Systemd服务文件

新建`srm.service`文件，路径：`/etc/systemd/system/srm.service`

```ini
[Unit]
Description=Safe RM Trash Auto Clean Service
After=network.target local-fs.target
Documentation=man:srm(1)

[Service]
Type=oneshot  # 单次执行，配合timer触发
ExecStart=/usr/local/bin/srm clean  # 执行清理过期项命令
User=root  # 若为普通用户使用，改为对应用户名（如user）
Group=root  # 对应用户组
WorkingDirectory=/tmp
Restart=no  # 无需重启
PrivateTmp=true  # 私有临时目录，提高安全性

[Install]
WantedBy=multi-user.target
```

### 步骤2：创建Systemd定时器文件

新建`srm.timer`文件，路径：`/etc/systemd/system/srm.timer`（控制执行周期）

```ini
[Unit]
Description=Timer for Safe RM Trash Auto Clean
Requires=srm.service

[Timer]
OnCalendar=daily  # 执行周期：每天执行（可自定义，如hourly/weekly）
Persistent=true   # 若系统关机错过执行，开机后自动补执行
AccuracySec=1min  # 执行精度：1分钟内
Unit=srm.service  # 关联的服务文件

[Install]
WantedBy=timers.target
```

### 步骤3：重载配置并启用服务

```bash
# 重载Systemd配置
sudo systemctl daemon-reload
# 启用并启动定时器（核心：开机自启）
sudo systemctl enable --now srm.timer
# 验证定时器状态
sudo systemctl list-timers srm.timer
```

### 关键命令

```bash
# 手动执行一次清理
sudo systemctl start srm.service
# 查看服务执行日志
journalctl -u srm.service -f
# 查看定时器状态
sudo systemctl status srm.timer
# 停止并禁用定时器
sudo systemctl disable --now srm.timer
```

### 周期自定义说明

修改`OnCalendar`参数可自定义执行周期，常见值：

- `hourly`：每小时执行
- `daily`：每天执行（默认）
- `weekly`：每周执行
- `monthly`：每月执行
- 自定义时间：`*-*-* 02:00:00`（每天凌晨2点执行）

## 安全替代原生rm（按用户独立生效）

实现**单个用户**使用`srm`替代原生`rm`，**不影响其他用户和系统全局`rm`**，核心通过Shell别名实现，支持`bash`/`zsh`，步骤如下：

### 方式1：临时生效（当前终端）

```bash
# bash/zsh 通用
alias rm='srm del'
# 可选：添加rm -f 映射为srm del -f
alias rmf='srm del -f'
```

- 执行后，当前终端中输入`rm test.txt`即等价于`srm del test.txt`
- 关闭终端后别名失效，不影响系统

### 方式2：永久生效（仅当前用户，推荐）

#### 对于bash用户

```bash
# 将别名写入bash配置文件
echo 'alias rm="srm del"' >> $HOME/.bashrc
echo 'alias rmf="srm del -f"' >> $HOME/.bashrc
# 加载配置生效
source $HOME/.bashrc
```

#### 对于zsh用户

```bash
# 将别名写入zsh配置文件
echo 'alias rm="srm del"' >> $HOME/.zshrc
echo 'alias rmf="srm del -f"' >> $HOME/.zshrc
# 加载配置生效
source $HOME/.zshrc
```

### 关键说明

1. **用户隔离**：仅当前用户的Shell生效，其他用户（包括root）仍使用原生`rm`
2. **系统安全**：系统级脚本/命令仍调用原生`rm`，不会因`srm`故障影响系统运行
3. **还原方法**：若需恢复原生`rm`，删除配置文件中的别名行即可：

   ```bash
   # bash
   sed -i '/alias rm=/d' $HOME/.bashrc
   source $HOME/.bashrc
   # zsh
   sed -i '/alias rm=/d' $HOME/.zshrc
   source $HOME/.zshrc
   ```

4. **兼容原生习惯**：保留`rm`的使用习惯，无需记忆新命令，降低学习成本

## 配置说明

### 核心配置特点

`srm`采用**硬编码默认配置+无外置配置文件**的设计（源码中通过常量定义），无需手动修改配置，开箱即用，核心默认配置均为行业最佳实践，源码中关键常量如下：

| 常量名 | 取值 | 说明 |
|--------|------|------|
| `DEFAULT_EXPIRE_DAYS` | 7 | 默认过期天数 |
| `MAX_LOG_AGE_DAYS` | 30 | 日志最大保留天数（自动轮转） |
| `PROTECTED_PATHS` | 8个系统路径 | 默认保护的核心系统路径 |
| `SHORT_ID_LENGTH` | 6 | 短ID字符长度 |
| `PROGRESS_THRESHOLD_BYTES` | 100MB | 显示进度条的文件大小阈值 |
| `MAX_FILE_SPACE_RATIO` | 0.8 | 单文件最大占用可用空间比例（80%） |
| `MMAP_CHUNK_SIZE` | 4MB | 大文件mmap分块大小 |

### 自定义配置（源码修改）

若需调整默认配置，修改`main.rs`中的常量后重新编译即可：

1. 打开`main.rs`，找到常量定义区域（文件顶部）
2. 修改对应常量值（如将`DEFAULT_EXPIRE_DAYS`改为15）
3. 重新编译：`cargo build --release`
4. 覆盖原二进制：`sudo cp target/release/srm /usr/local/bin/`

### 数据存储目录

所有核心数据均存储在**`srm`可执行文件同级的`.srm`目录**中，目录结构：

```
.srm/
├── trash/        # 回收站：存储被删除的文件/目录
├── meta/         # 元数据：存储每个删除项的JSON格式元数据（原子化写入）
└── srm.log       # 审计日志：JSON格式，自动轮转（30天保留）
```

- 目录权限：`0700`，文件权限：`0600`，仅当前用户可访问
- 若需迁移回收站数据，直接复制整个`.srm`目录即可

## 性能优化特性

源码针对**大文件/大目录/跨文件系统操作**做了多层极致优化，适配TB级文件操作，核心优化点如下：

1. **同文件系统即时重命名**：若源文件和回收站在同一文件系统，直接执行`rename`系统调用，**0拷贝**，瞬间完成
2. **跨文件系统写时复制（CoW）**：Linux下自动检测Btrfs/XFS/ZFS等支持CoW的文件系统，通过`ioctl FICLONE`实现**无数据拷贝**的复制，比普通拷贝快10倍以上
3. **硬链接优先**：同文件系统下，若CoW不支持，自动尝试创建硬链接，避免数据拷贝
4. **大文件内存映射I/O（mmap）**：对>10MB的文件，使用`mmap2`将文件映射到内存，分4MB块传输，减少系统调用，提高吞吐量
5. **迭代式目录遍历**：采用栈实现目录迭代遍历，**避免递归栈溢出**，支持最大1000级目录深度
6. **分块进度追踪**：大文件/目录操作时，按块更新进度条，实时显示耗时、吞吐量、剩余时间
7. **批量磁盘空间校验**：删除前批量校验磁盘空间，避免多次IO操作，提高批量删除效率
8. **Rust编译优化**：`Cargo.toml`中开启`opt-level=3`、`lto=fat`、`strip=true`，编译出的二进制体积小、执行效率高

## 日志与审计

### 日志特点

1. **JSON格式**：所有日志均为标准JSON格式，便于自动化解析、审计和日志收集工具（如ELK）对接
2. **自动轮转**：日志保留30天，自动清理30天前的日志，避免日志文件过大
3. **权限隔离**：日志文件权限`0600`，仅当前用户可读取，防止审计数据泄露
4. **全量记录**：记录所有操作类型，包括删除、恢复、清理、空回收站、操作中断、跳过/失败等
5. **元数据完整**：每条日志包含**时间戳（毫秒级）、日志级别、操作信息、详细元数据**（路径、短ID、大小、权限、UID/GID、执行结果等）

### 日志路径

```bash
# 日志文件位于.srm目录中，路径：
$(dirname $(which srm))/.srm/srm.log
# 示例：若srm在/usr/local/bin/，则日志路径为/usr/local/bin/.srm/srm.log
```

### 日志内容示例

```json
{
  "timestamp": "2026-01-30 15:20:30.123",
  "level": "INFO",
  "message": "File deleted",
  "details": {
    "action": "delete",
    "short_id": "f_a3b4c5",
    "trash_id": "test.txt_1738238430123456789",
    "original_path": "/home/user/test.txt",
    "backup_path": "/usr/local/bin/.srm/trash/test.txt_1738238430123456789",
    "file_type": "file",
    "size_bytes": 1258291,
    "permissions": "644",
    "expire_days": 7,
    "forced": false,
    "duration_ms": 120
  }
}
```

### 日志查看与解析

```bash
# 实时查看日志
tail -f $(dirname $(which srm))/.srm/srm.log
# 格式化查看JSON日志（需安装jq）
jq . $(dirname $(which srm))/.srm/srm.log
# 筛选删除操作日志
jq 'select(.details.action == "delete")' $(dirname $(which srm))/.srm/srm.log
```

## 安全防护机制

`srm`内置多层安全防护机制，从根本上避免误删和系统损坏，核心防护点如下：

1. **系统路径保护**：默认禁止删除`/bin`、`/sbin`、`/etc`、`/usr`、`/lib`、`/lib64`、`/root`、`/boot`8个核心系统路径，需`-f`强制覆盖
2. **路径遍历防护**：默认禁止删除含`..`的路径（如`../etc/passwd`），防止路径遍历攻击，需`-f`强制覆盖
3. **磁盘空间保护**：删除前检查目标文件系统可用空间，单文件最大占用80%可用空间，批量删除校验总空间，防止磁盘占满
4. **权限严格隔离**：回收站、日志、元数据均为`0700/0600`权限，仅当前用户可访问，避免越权查看/修改/恢复文件
5. **软链目标校验**：检查软链指向的目标路径，若指向系统保护路径，默认禁止删除，需`-f`强制覆盖
6. **中断安全**：Ctrl+C中断操作时，正在执行的删除任务会自动回滚，将已复制的文件恢复到原路径，避免文件丢失
7. **原子化元数据**：元数据采用“先写临时文件，再重命名”的原子化操作，防止进程崩溃导致元数据损坏
8. **不存在文件跳过**：删除时自动跳过不存在的文件，不抛出错误，提高批量操作稳定性

## 常见问题

### Q1：删除的文件存储在哪里？如何迁移回收站数据？

A：存储在`srm`可执行文件同级的`.srm/trash`目录中；迁移时直接复制整个`.srm`目录到新的`srm`可执行文件同级即可，元数据和日志会自动保留。

### Q2：忘记短ID了，如何恢复文件？

A：执行`srm ls`查看回收站所有项的短ID和原路径，找到对应项后用`srm res 短ID`恢复即可。

### Q3：srm是否支持跨文件系统删除？

A：支持，跨文件系统时会自动采用**CoW写时复制**（支持的文件系统）或**mmap分块复制**，并显示实时进度条，性能优于原生`mv`。

### Q4：为什么执行`srm del`后，原文件路径的磁盘空间没有释放？

A：因为`srm`是将文件移动到回收站，并非永久删除，磁盘空间会在执行`srm cln`（清理过期）或`srm empty`（清空）后释放。

### Q5：普通用户能否删除root用户的文件？

A：不能，受Linux文件系统权限控制，普通用户仅能删除自己拥有读写权限的文件，与原生`rm`一致。

### Q6：srm是否支持大文件（如100GB）删除？

A：支持，针对大文件做了**mmap分块传输**和**实时进度追踪**，支持中断回滚，不会因内存不足导致崩溃。

### Q7：如何查看srm的操作日志？

A：日志路径为`$(dirname $(which srm))/.srm/srm.log`，可通过`tail -f`实时查看，或用`jq`格式化解析。

### Q8：Systemd定时器不执行怎么办？

A：1. 检查定时器状态：`sudo systemctl status srm.timer`；
2. 查看执行日志：`journalctl -u srm.service -f`；
3. 确认`srm`路径正确（`/usr/local/bin/srm`）；
4. 重新重载配置：`sudo systemctl daemon-reload && sudo systemctl restart srm.timer`。

---

**版本**：v1.2.1  
**作者**：Meitao Lin <mtl>  
**许可证**：MIT  
**开发语言**：Rust 2021 Edition  
**构建工具**：Cargo 1.93.0 (083ac5135 2025-12-15)
