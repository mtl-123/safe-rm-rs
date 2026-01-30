# safe-rm-rs 项目 GitHub Actions 全流程构建发布操作手册

# 一、前言

本文档整理 safe-rm-rs 项目通过 GitHub Actions 实现「静态编译+UPX压缩+自动发布」的全流程操作步骤，包含前期配置、旧版本清理、重新发布、功能验证，确保所有操作可复现、无遗漏，最终实现 Releases 页面正常显示功能完整的二进制文件。

核心目标：通过 GitHub Actions 自动构建 Linux x86_64 静态二进制文件（启用 UPX --best --lzma 压缩），推送 Tag 后自动发布到 GitHub Releases，确保下载的二进制文件可正常执行。

# 二、前期准备

## 2.1 环境要求

- 本地环境：安装 Git、Rust（可选，用于本地验证）

- GitHub 权限：拥有 safe-rm-rs 仓库的读写权限（可推送代码、删除 Tag、发布 Releases）

- 网络环境：可正常访问 GitHub（避免 Actions 构建失败）

## 2.2 核心前提

- 本地代码已同步 GitHub 最新 master 分支，且无冲突

- 已配置 GitHub SSH 免密（参考前期 SSH 配置步骤，避免推送代码时反复输入密码）

# 三、核心配置（rust.yml 完整配置）

替换/创建项目根目录下的 `.github/workflows/rust.yml` 文件，配置静态编译、UPX 压缩、自动发布全流程，内容如下：

```yaml

# 工作流名称
name: Build & Release Static Binary

# 触发条件：master分支推送、Tag推送（v开头）、手动触发
on:
  push:
    branches: [master]
    tags: ['v*']
  workflow_dispatch:

# 核心权限配置（发布Releases必需）
permissions:
  contents: write
  packages: write
  actions: read

# 构建任务
jobs:
  build-linux-x86_64-static:
    name: Build Linux x86_64 Static Binary
    runs-on: ubuntu-22.04
    steps:
      # 步骤1：拉取源码
      - name: Checkout Source Code
        uses: actions/checkout@v4
        with:
          fetch-depth: 1

      # 步骤2：安装Rust工具链（含musl静态编译目标）
      - name: Install Rust Toolchain
        uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: stable
          target: x86_64-unknown-linux-musl  # 静态编译目标
          components: rustfmt, clippy

      # 步骤3：安装系统依赖（静态编译 + UPX压缩必需）
      - name: Install System Dependencies
        run: |
          sudo apt update -y
          sudo apt install -y build-essential musl-tools upx  # 安装UPX压缩工具
          # 验证依赖安装成功
          musl-gcc --version
          upx --version

      # 步骤4：静态编译二进制 + UPX压缩（核心步骤）
      - name: Build Static Optimized Binary & UPX Compress
        run: |
          # 静态编译：启用CPU原生优化+静态链接+发布模式
          RUSTFLAGS="-C target-cpu=native -C link-arg=-static" \
          cargo build --release --target x86_64-unknown-linux-musl
          
          # 强制验证二进制文件是否生成（失败则终止流程）
          echo "=== 验证编译产物 ==="
          ls -lh target/x86_64-unknown-linux-musl/release/
          if [ ! -f "target/x86_64-unknown-linux-musl/release/safe-rm" ]; then
            echo "错误：二进制文件未生成！"
            exit 1
          fi
          
          # 验证静态链接（确保无系统依赖）
          file target/x86_64-unknown-linux-musl/release/safe-rm
          # 正常输出应包含：statically linked

          # UPX压缩（使用指定参数：--best --lzma）
          echo "=== UPX压缩二进制 ==="
          upx --best --lzma target/x86_64-unknown-linux-musl/release/safe-rm
          # 验证压缩后文件可执行（非强制，避免兼容问题终止流程）
          chmod +x target/x86_64-unknown-linux-musl/release/safe-rm
          ./target/x86_64-unknown-linux-musl/release/safe-rm --help || echo "UPX压缩后功能验证（非强制）"

      # 步骤5：打包二进制+生成校验和
      - name: Package Binary & Generate Checksum
        run: |
          mkdir -p release
          
          # 版本号处理：Tag推送用Tag名，非Tag用dev+提交哈希
          VERSION="${{ github.ref_name }}"
          if [[ "${{ github.ref }}" != refs/tags/* ]]; then
            VERSION="dev-$(git rev-parse --short HEAD)"
          fi
          
          # 复制并命名静态二进制（保留原有命名规范）
          BIN_NAME="safe-rm-${VERSION}-linux-x86_64-static"
          cp target/x86_64-unknown-linux-musl/release/safe-rm release/${BIN_NAME}
          
          # 生成SHA256校验和（用户可验证文件完整性）
          sha256sum release/${BIN_NAME} > release/${BIN_NAME}.sha256
          
          # 输出打包结果（便于日志排查）
          echo "=== 最终打包文件 ==="
          ls -lh release/
          cat release/${BIN_NAME}.sha256

      # 步骤6：上传Artifact（便于手动下载）
      - name: Upload Binary to Artifact
        uses: actions/upload-artifact@v4
        with:
          name: safe-rm-linux-x86_64-static-${{ github.ref_name }}
          path: release/
          retention-days: 30  #  artifacts保留30天

      # 步骤7：发布到GitHub Releases（仅Tag推送时执行）
      - name: Publish to GitHub Releases
        if: startsWith(github.ref, 'refs/tags/')
        uses: softprops/action-gh-release@v2
        with:
          files: |
            release/safe-rm-${{ github.ref_name }}-linux-x86_64-static
            release/safe-rm-${{ github.ref_name }}-linux-x86_64-static.sha256
          generate_release_notes: true  # 自动生成发布说明
          prerelease: false             # 标记为正式发布
          name: "Release ${{ github.ref_name }} (Static Binary)"
          tag_name: ${{ github.ref_name }}

  # 可选：Windows x86_64 构建（如需可取消注释）
  # build-windows-x86_64:
  #   name: Build Windows x86_64 Binary
  #   runs-on: windows-2022
  #   steps:
  #     - name: Checkout Source Code
  #       uses: actions/checkout@v4
  #     - name: Install Rust Toolchain
  #       uses: dtolnay/rust-toolchain@v1
  #       with:
  #         toolchain: stable
  #         target: x86_64-pc-windows-msvc
  #     - name: Build Binary
  #       run: cargo build --release --target x86_64-pc-windows-msvc
  #     - name: Package Binary
  #       run: |
  #         mkdir -p release
  #         $VERSION="${{ github.ref_name }}"
  #         if (-not $env:GITHUB_REF.StartsWith('refs/tags/')) {
  #           $VERSION="dev-$(git rev-parse --short HEAD)"
  #         }
  #         Copy-Item target/x86_64-pc-windows-msvc/release/safe-rm.exe release/safe-rm-${VERSION}-windows-x86_64.exe
  #         Get-FileHash -Algorithm SHA256 release/safe-rm-${VERSION}-windows-x86_64.exe | Format-List | Out-File release/safe-rm-${VERSION}-windows-x86_64.sha256
  #     - name: Upload Artifact
  #       uses: actions/upload-artifact@v4
  #       with:
  #         name: safe-rm-windows-x86_64-${{ github.ref_name }}
  #         path: release/
  #     - name: Publish to GitHub Releases
  #       if: startsWith(github.ref, 'refs/tags/')
  #       uses: softprops/action-gh-release@v2
  #       with:
  #         files: |
  #           release/safe-rm-${{ github.ref_name }}-windows-x86_64.exe
  #           release/safe-rm-${{ github.ref_name }}-windows-x86_64.sha256
```

# 四、全流程操作步骤（实操）

## 4.1 步骤1：配置文件部署

将上述完整配置覆盖本地项目的 `.github/workflows/rust.yml` 文件，执行以下命令：

```bash

# 进入项目根目录
cd /path/to/your/safe-rm-rs

# 覆盖/创建rust.yml配置文件（确保内容完整）
cat > .github/workflows/rust.yml << 'EOF'
# 粘贴上面的完整yaml配置内容（复制整个rust.yml代码）
EOF
```

## 4.2 步骤2：提交并推送配置文件

推送配置文件到 GitHub master 分支，确保配置生效，同时避免冲突（保留本地修改）：

```bash

# 查看修改状态（确认只有rust.yml文件变更）
git status

# 暂存修改
git add .github/workflows/rust.yml

# 提交修改（备注清晰，便于后续追溯）
git commit -m "feat: 配置静态编译+UPX压缩，完善Actions全流程构建发布"

# 拉取远程最新代码（保留本地修改，避免推送被拒绝）
git pull origin master -X ours

# 推送配置到远程master分支
git push origin master
```

## 4.3 步骤3：清理旧版本（可选，若有旧Tag/Releases）

若之前有失败的发布版本（如 v1.0.1、v1.0.2 等），需彻底清理本地+远程Tag，确保重新发布的版本干净无冗余：

```bash

# 1. 列出本地所有Tag（确认要删除的Tag，如v1.0.1、v1.0.2等）
git tag

# 2. 单个删除本地Tag（推荐，避免误删）
git tag -d v1.0.1
git tag -d v1.0.2  # 如有其他旧Tag，依次删除

# 查询远程仓库版本号
git ls-remote --tags origin

# 3. 单个删除远程Tag（同步清理GitHub Releases页面的旧版本）
git push origin --delete v1.0.1
git push origin --delete v1.0.2  # 对应删除远程Tag

# 注意：若需清理测试分支（非master），执行以下命令（替换为你的分支名）
# git push origin --delete 测试分支名
# 严禁删除master分支！
```

## 4.4 步骤4：重新创建Tag并推送（触发Actions）

创建目标版本Tag（本文以 v1.0.1 为例，必须以 v 开头），推送Tag后自动触发 GitHub Actions 构建和发布：

```bash

# 1. 创建带注释的Tag（-a 表示带注释，-m 是注释内容，标注版本特性）
git tag -a v1.0.1 -m "Release v1.0.1 (静态编译+UPX --best --lzma压缩，功能完整)"

# 2. 推送Tag到GitHub（触发Actions工作流）
git push origin v1.0.1
```

## 4.5 步骤5：监控Actions构建进度

推送Tag后，进入 GitHub 仓库，监控 Actions 构建过程，确保所有步骤执行成功：

1. 打开 safe-rm-rs 仓库页面 → 点击顶部「Actions」标签；

2. 找到对应 Tag（v1.0.1）的运行记录（标题含「Build Linux x86_64 Static Binary」）；

3. 点击运行记录，查看每一步执行状态，所有步骤需显示绿色对勾 ✅（无标红失败）；

4. 重点检查3个核心步骤：
        

    - 「Install System Dependencies」：确认 musl-tools、upx 安装成功；

    - 「Build Static Optimized Binary & UPX Compress」：确认二进制生成、静态链接验证通过、UPX压缩成功；

    - 「Publish to GitHub Releases」：确认成功上传二进制文件和校验和文件。

若某一步失败，点击该步骤，查看日志排查问题（常见问题：UPX压缩失败、权限不足，参考文档末尾排查方案）。

# 五、验证发布结果（关键步骤）

Actions 构建完成后，需验证 Releases 页面文件完整性和二进制功能正常，分3步验证：

## 5.1 验证Releases页面文件

1. 打开 safe-rm-rs 仓库 → 点击顶部「Releases」标签；

2. 找到 v1.0.1 版本，确认页面包含以下2个文件：
        

    - safe-rm-v1.0.1-linux-x86_64-static（静态二进制文件）；

    - safe-rm-v1.0.1-linux-x86_64-static.sha256（SHA256校验和文件）。

3. 确认发布标题、发布说明正常显示，无空发布情况。

## 5.2 验证二进制文件完整性

下载 Releases 中的2个文件，验证文件完整性（可选，确保文件未损坏）：

```bash

# 1. 下载文件（可通过浏览器下载，或用wget命令）
wget https://github.com/mtl-123/safe-rm-rs/releases/download/v1.0.1/safe-rm-v1.0.1-linux-x86_64-static
wget https://github.com/mtl-123/safe-rm-rs/releases/download/v1.0.1/safe-rm-v1.0.1-linux-x86_64-static.sha256

# 2. 验证校验和（确保文件未损坏）
sha256sum -c safe-rm-v1.0.1-linux-x86_64-static.sha256
# 正常输出：safe-rm-v1.0.1-linux-x86_64-static: OK
```

## 5.3 验证二进制功能正常

赋予二进制文件执行权限，测试核心功能，确保无异常：

```bash

# 1. 赋予执行权限
chmod +x safe-rm-v1.0.1-linux-x86_64-static

# 2. 验证静态链接（无系统依赖，确保任意Linux可运行）
ldd safe-rm-v1.0.1-linux-x86_64-static
# 正常输出：not a dynamic executable

# 3. 测试核心功能（根据项目逻辑，示例为查看帮助）
./safe-rm-v1.0.1-linux-x86_64-static --help
# 正常输出：项目帮助信息，无报错、无空白输出
```

# 六、常见问题排查

## 6.1 Actions 构建失败

- 问题1：UPX 压缩失败 → 检查 UPX 安装是否成功，或临时注释 UPX 压缩步骤，重新构建；

- 问题2：二进制未生成 → 检查 Rust 工具链安装步骤，确认 target 为 x86_64-unknown-linux-musl；

- 问题3：权限不足 → 确认 rust.yml 中 permissions 配置包含 contents: write。

## 6.2 二进制文件执行无输出

- 原因：UPX 压缩过度损坏二进制 → 注释 UPX 压缩步骤，重新构建验证；

- 原因：代码逻辑问题 → 本地编译测试（cargo build --release），确认 main 函数有输出、参数解析正常。

## 6.3 推送 Tag 后未触发 Actions

检查 Tag 格式是否以 v 开头（如 v1.0.1，非 1.0.1），重新创建 Tag 并推送。

# 七、总结

全流程核心逻辑：配置 rust.yml（静态编译+UPX压缩）→ 推送配置文件 → 清理旧版本 → 创建Tag触发Actions → 验证发布结果。

执行完所有步骤后，GitHub Releases 页面会显示功能完整、体积优化的静态二进制文件，用户可直接下载使用，无需手动编译；后续发布新版本，仅需重复「创建Tag→推送Tag」步骤即可自动触发构建发布。
> （注：文档部分内容可能由 AI 生成）
