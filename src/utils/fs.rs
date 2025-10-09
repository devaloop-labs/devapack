use std::fs;
use std::path::{Path, PathBuf};

pub fn get_cwd() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|e| format!("Failed to get current working directory: {}", e))
}

pub fn get_user_home() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "Failed to get user home directory".to_string())
}

pub fn walk_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    let mut files: Vec<PathBuf> = Vec::new();
    while let Some(dir) = stack.pop() {
        let rd = fs::read_dir(&dir).map_err(|e| format!("Failed to read directory: {}", e))?;
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.is_file() {
                files.push(p);
            }
        }
    }
    Ok(files)
}

pub fn path_relative_to(path: &Path, base: &Path) -> Option<PathBuf> {
    let rel = path.strip_prefix(base).ok()?;
    Some(rel.to_path_buf())
}

pub fn to_unix_string<P: AsRef<Path>>(p: P) -> String {
    let s = p.as_ref().to_string_lossy().into_owned();
    s.replace('\\', "/")
}

pub fn is_ignored_component(name: &str) -> bool {
    matches!(
        name,
        "node_modules" | ".git" | "target" | "dist" | "build" | "out"
    )
}
