// 核心依赖导入
use clap::{Parser, Subcommand};
use chrono::{Duration, Local, NaiveDateTime, TimeZone};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// 配置常量
const DEFAULT_EXPIRE_DAYS: i64 = 7;
const TRASH_DIR: &str = ".safe-rm/trash";
const META_FILE: &str = ".safe-rm/metadata.json";

#[derive(Parser, Debug)]
#[command(
    author = "Your Name",
    version = "1.0.0",
    about = "Safe rm tool with restore/auto-clean (compatible with rm habits)",
    long_about = "A safe alternative to rm: move files to trash instead of permanent delete, support restore, auto-clean, recursive dir handling.\n\nExamples:\n  safe-rm del test.txt          # Delete file (7 days expire)\n  safe-rm res test.txt_123456  # Restore deleted file\n  safe-rm ls                   # List trash files\n  safe-rm cln                  # Clean expired files"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(alias = "del", about = "Delete file/dir (recursive support)", long_about = "Move file/directory to trash (include nested files/dirs).\nExamples:\n  safe-rm delete test.txt                # Delete single file\n  safe-rm del /home/user/test_dir        # Delete directory (recursive)\n  safe-rm delete --expire-days 3 file1.txt file2.txt  # Batch delete\n  safe-rm del -f /tmp/system_test        # Force delete (skip system check)")]
    Delete {
        #[arg(required = true, help = "Path(s) to delete (file/dir, support multiple)")]
        paths: Vec<PathBuf>,
        #[arg(short = 'd', long, default_value_t = DEFAULT_EXPIRE_DAYS, help = "Expire days (default: 7)")]
        expire_days: i64,
        #[arg(short = 'f', long, help = "Force delete (skip system dir check)")]
        force: bool,
    },

    #[command(alias = "res", about = "Restore deleted file/dir", long_about = "Restore file/directory from trash to original path.\nExamples:\n  safe-rm restore test.txt_123456        # Restore single file\n  safe-rm res dir_123456 file_123457    # Batch restore\n  safe-rm restore --force dir_123456    # Force restore (overwrite existing file)")]
    Restore {
        #[arg(required = true, help = "Trash name(s) (from `safe-rm list`)")]
        names: Vec<String>,
        #[arg(short = 'f', long, help = "Force restore (overwrite existing)")]
        force: bool,
    },

    #[command(alias = "ls", about = "List trash files/dirs", long_about = "Show all files in trash with detail info.\nExamples:\n  safe-rm list                # List all trash files\n  safe-rm ls --expired        # Only show expired files")]
    List {
        #[arg(long, help = "Only show expired files")]
        expired: bool,
    },

    #[command(alias = "cln", about = "Clean expired files", long_about = "Manually clean all expired files in trash.\nExamples:\n  safe-rm clean               # Clean expired files\n  safe-rm cln --all           # Clean ALL trash files (no expire check)")]
    Clean {
        #[arg(short = 'a', long, help = "Clean ALL trash files (CAUTION!)")]
        all: bool,
    },

    #[command(alias = "exp", about = "Check expire time", long_about = "Show expire time of a specific trash file.\nExamples:\n  safe-rm expire test.txt_123456        # Check expire time")]
    Expire {
        #[arg(required = true, help = "Trash name to check")]
        name: String,
    },

    #[command(alias = "empty", about = "Empty trash", long_about = "Permanently delete all files in trash.\nExamples:\n  safe-rm empty               # Empty trash (need confirm)\n  safe-rm empty --yes         # Empty without confirm")]
    Empty {
        #[arg(short = 'y', long, help = "Skip confirmation")]
        yes: bool,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FileMeta {
    original_path: PathBuf,
    trash_path: PathBuf,
    delete_time: String,
    expire_days: i64,
    is_dir: bool,
}

// 递归复制目录
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

// 递归恢复目录（从 trash 到原位置）
fn restore_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            restore_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn main() {
    let home = home_dir().expect("Failed to get home directory");
    let trash_dir = home.join(TRASH_DIR);
    let meta_file = home.join(META_FILE);
    fs::create_dir_all(&trash_dir).expect("Failed to create trash directory");

    let mut metadata = load_metadata(&meta_file);

    // 自动清理过期文件
    clean_expired_files(&mut metadata, false);
    save_metadata(&metadata, &meta_file);

    let cli = Cli::parse();
    match cli.cmd {
        Commands::Delete { paths, expire_days, force } => {
            for path in paths {
                handle_delete(&path, expire_days, force, &trash_dir, &mut metadata);
            }
        }
        Commands::Restore { names, force } => {
            for name in names {
                handle_restore(&name, force, &mut metadata);
            }
        }
        Commands::List { expired } => {
            handle_list(&metadata, expired);
        }
        Commands::Clean { all } => {
            clean_expired_files(&mut metadata, all);
            println!("Clean completed!");
        }
        Commands::Expire { name } => {
            handle_expire_check(&name, &metadata);
        }
        Commands::Empty { yes } => {
            handle_empty_trash(yes, &mut metadata);
        }
    }

    save_metadata(&metadata, &meta_file);
}

fn load_metadata(meta_file: &Path) -> HashMap<String, FileMeta> {
    if meta_file.exists() {
        let content = fs::read_to_string(meta_file).unwrap_or_else(|_| {
            eprintln!("Warning: Failed to read metadata, create new");
            "{}".to_string()
        });
        serde_json::from_str(&content).unwrap_or_else(|_| {
            eprintln!("Warning: Metadata corrupted, reset to empty");
            HashMap::new()
        })
    } else {
        HashMap::new()
    }
}

fn save_metadata(metadata: &HashMap<String, FileMeta>, meta_file: &Path) {
    let content = serde_json::to_string_pretty(metadata).expect("Failed to serialize metadata");
    fs::write(meta_file, content).expect("Failed to write metadata");
}

fn handle_delete(
    path: &Path,
    expire_days: i64,
    force: bool,
    trash_dir: &Path,
    metadata: &mut HashMap<String, FileMeta>,
) {
    let system_paths = ["/bin", "/sbin", "/etc", "/usr", "/lib", "/lib64", "/root", "/boot"];
    let path_abs = match fs::canonicalize(path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("⚠️  Skip '{}': Failed to get absolute path: {}", path.display(), e);
            return;
        }
    };

    if !path_abs.exists() {
        eprintln!("⚠️  Skip '{}': File/dir not found", path.display());
        return;
    }

    let path_str = path_abs.to_str().unwrap_or("");
    if !force && system_paths.iter().any(|p| path_str.starts_with(p)) {
        eprintln!("❌  Skip '{}': System directory protected (use -f to force)", path.display());
        return;
    }

    let is_dir = path_abs.is_dir();
    let file_name = path_abs.file_name().unwrap().to_str().unwrap();
    let timestamp = Local::now().timestamp();
    let trash_file_name = format!("{}_{}", file_name, timestamp);
    let trash_path = trash_dir.join(&trash_file_name);

    println!("📤 Moving '{}' to trash (expire in {} days)...", path.display(), expire_days);

    let success = if is_dir {
        // 复制整个目录
        if let Err(e) = copy_dir_all(&path_abs, &trash_path) {
            eprintln!("❌  Failed to copy dir '{}': {}", path.display(), e);
            return;
        }
        // 删除原目录
        if let Err(e) = fs::remove_dir_all(&path_abs) {
            // 回滚：删除 trash 中的副本
            let _ = fs::remove_dir_all(&trash_path);
            eprintln!("❌  Failed to remove original dir '{}': {}", path.display(), e);
            return;
        }
        true
    } else {
        // 复制文件
        if let Err(e) = fs::copy(&path_abs, &trash_path) {
            eprintln!("❌  Failed to copy file '{}': {}", path.display(), e);
            return;
        }
        // 删除原文件
        if let Err(e) = fs::remove_file(&path_abs) {
            let _ = fs::remove_file(&trash_path);
            eprintln!("❌  Failed to remove original file '{}': {}", path.display(), e);
            return;
        }
        true
    };

    if success {
        println!("✅  Moved to trash: {}", trash_path.display());
        let delete_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        metadata.insert(
            trash_file_name,
            FileMeta {
                original_path: path_abs,
                trash_path,
                delete_time,
                expire_days,
                is_dir,
            },
        );
    }
}

fn handle_restore(
    name: &str,
    force: bool,
    metadata: &mut HashMap<String, FileMeta>,
) {
    let Some(meta) = metadata.remove(name) else {
        eprintln!("❌  Restore '{}': Not found in trash", name);
        return;
    };

    if !meta.trash_path.exists() {
        eprintln!("❌  Restore '{}': Trash file not found", name);
        metadata.insert(name.to_string(), meta);
        return;
    }

    if meta.original_path.exists() && !force {
        eprintln!("❌  Restore '{}': Target '{}' exists (use -f to force)", name, meta.original_path.display());
        metadata.insert(name.to_string(), meta);
        return;
    }

    // 如果目标存在且 force=true，先删除
    if force && meta.original_path.exists() {
        let _ = if meta.is_dir {
            fs::remove_dir_all(&meta.original_path)
        } else {
            fs::remove_file(&meta.original_path)
        };
    }

    println!("🔄 Restoring '{}' to '{}'...", name, meta.original_path.display());

    let success = if meta.is_dir {
        if let Err(e) = restore_dir_all(&meta.trash_path, &meta.original_path) {
            eprintln!("❌  Failed to restore dir '{}': {}", name, e);
            false
        } else {
            // 删除 trash 中的副本
            fs::remove_dir_all(&meta.trash_path).is_ok()
        }
    } else {
        if let Err(e) = fs::copy(&meta.trash_path, &meta.original_path) {
            eprintln!("❌  Failed to restore file '{}': {}", name, e);
            false
        } else {
            fs::remove_file(&meta.trash_path).is_ok()
        }
    };

    if !success {
        eprintln!("❌  Failed to complete restore '{}'", name);
        // 不加回元数据（因为 trash 文件可能已部分删除）
        return;
    }

    println!("✅  Restored: {}", meta.original_path.display());
}

fn handle_list(metadata: &HashMap<String, FileMeta>, expired: bool) {
    if metadata.is_empty() {
        println!("🗑️  Trash is empty");
        return;
    }

    println!("=== Safe-RM Trash List (Total: {}) ===", metadata.len());
    let now = Local::now();

    for (name, meta) in metadata {
        if expired {
            let delete_time = NaiveDateTime::parse_from_str(&meta.delete_time, "%Y-%m-%d %H:%M:%S").unwrap();
            let delete_time = Local.from_local_datetime(&delete_time).unwrap();
            let expire_time = delete_time + Duration::days(meta.expire_days);
            if now <= expire_time {
                continue;
            }
        }

        let delete_time = NaiveDateTime::parse_from_str(&meta.delete_time, "%Y-%m-%d %H:%M:%S").unwrap();
        let delete_time = Local.from_local_datetime(&delete_time).unwrap();
        let expire_time = delete_time + Duration::days(meta.expire_days);
        let remaining = expire_time.signed_duration_since(now);
        let remaining_str = if remaining.num_days() > 0 {
            format!("{} days left", remaining.num_days())
        } else if remaining.num_hours() > 0 {
            format!("{} hours left", remaining.num_hours())
        } else {
            "Expired".to_string()
        };

        println!("▶ Name: {}", name);
        println!("  Type: {}", if meta.is_dir { "Directory" } else { "File" });
        println!("  Original: {}", meta.original_path.display());
        println!("  Delete Time: {}", meta.delete_time);
        println!("  Expire: {} ({})", meta.expire_days, remaining_str);
        println!("---------------------------");
    }
}

fn clean_expired_files(metadata: &mut HashMap<String, FileMeta>, clean_all: bool) {
    let now = Local::now();
    let mut to_remove = Vec::new();

    for (name, meta) in metadata.iter() {
        if clean_all {
            to_remove.push(name.clone());
            continue;
        }

        let delete_time = match NaiveDateTime::parse_from_str(&meta.delete_time, "%Y-%m-%d %H:%M:%S") {
            Ok(t) => t,
            Err(e) => {
                eprintln!("⚠️  Clean '{}': Invalid delete time: {}", name, e);
                to_remove.push(name.clone());
                continue;
            }
        };

        let delete_time = Local.from_local_datetime(&delete_time).unwrap();
        let expire_time = delete_time + Duration::days(meta.expire_days);
        if now > expire_time {
            to_remove.push(name.clone());
        }
    }

    for name in to_remove {
        if let Some(meta) = metadata.get(&name) {
            if meta.trash_path.exists() {
                let _ = if meta.is_dir {
                    fs::remove_dir_all(&meta.trash_path)
                } else {
                    fs::remove_file(&meta.trash_path)
                };
            }
            metadata.remove(&name);
            println!("🗑️  Cleaned: {}", name);
        }
    }
}

fn handle_expire_check(name: &str, metadata: &HashMap<String, FileMeta>) {
    let Some(meta) = metadata.get(name) else {
        eprintln!("❌  '{}' not found in trash", name);
        return;
    };

    let delete_time = NaiveDateTime::parse_from_str(&meta.delete_time, "%Y-%m-%d %H:%M:%S").unwrap();
    let delete_time = Local.from_local_datetime(&delete_time).unwrap();
    let expire_time = delete_time + Duration::days(meta.expire_days);
    let now = Local::now();
    let remaining = expire_time.signed_duration_since(now);

    println!("=== Expire Info for '{}' ===", name);
    println!("Original Path: {}", meta.original_path.display());
    println!("Delete Time: {}", meta.delete_time);
    println!("Expire Time: {}", expire_time.format("%Y-%m-%d %H:%M:%S"));
    if remaining.num_seconds() > 0 {
        println!("Remaining Time: {} days, {} hours", remaining.num_days(), remaining.num_hours() % 24);
    } else {
        println!("Status: Expired ({} hours ago)", -remaining.num_hours());
    }
}

fn handle_empty_trash(yes: bool, metadata: &mut HashMap<String, FileMeta>) {
    if !yes {
        print!("⚠️  Are you sure to empty trash (permanent delete all)? [y/N]: ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("✅  Canceled");
            return;
        }
    }

    for (name, meta) in metadata.drain() {
        if meta.trash_path.exists() {
            let _ = if meta.is_dir {
                fs::remove_dir_all(&meta.trash_path)
            } else {
                fs::remove_file(&meta.trash_path)
            };
        }
        println!("🗑️  Deleted: {}", name);
    }

    println!("✅  Trash emptied completely!");
}
