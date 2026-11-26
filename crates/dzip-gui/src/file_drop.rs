#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_native_dropped_paths(
    paths: Vec<(std::path::PathBuf, String)>,
) -> Result<Vec<DroppedFile>, String> {
    let mut dropped = Vec::new();
    for (path, name) in paths {
        if path.is_dir() {
            collect_native_directory(&path, &mut dropped)?;
        } else if path.is_file() {
            dropped.push(DroppedFile {
                path: name,
                bytes: std::fs::read(&path)
                    .map_err(|error| format!("无法读取 {}：{error}", path.display()))?,
            });
        } else {
            return Err(format!("无法读取不存在的拖放路径：{}", path.display()));
        }
    }
    dropped.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(dropped)
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn normalise_drop_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_matches('/')
        .to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_native_directory(
    root: &std::path::Path,
    output: &mut Vec<DroppedFile>,
) -> Result<(), String> {
    let root_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| "folder".to_string());
    collect_native_directory_entries(root, root, &root_name, output)?;
    output.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_native_directory_entries(
    root: &std::path::Path,
    directory: &std::path::Path,
    root_name: &str,
    output: &mut Vec<DroppedFile>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("无法读取文件夹 {}：{error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("无法读取 {} 的类型：{error}", path.display()))?;
        if file_type.is_dir() {
            collect_native_directory_entries(root, &path, root_name, output)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .ok()
                .map(|path| normalise_drop_path(&path.to_string_lossy()))
                .filter(|path| !path.is_empty())
                .unwrap_or_else(|| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map(str::to_string)
                        .unwrap_or_else(|| path.display().to_string())
                });
            output.push(DroppedFile {
                path: format!("{root_name}/{relative}"),
                bytes: std::fs::read(&path)
                    .map_err(|error| format!("无法读取 {}：{error}", path.display()))?,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn normalises_dropped_paths() {
        assert_eq!(normalise_drop_path(r".\Data\file.bin"), "Data/file.bin");
        assert_eq!(normalise_drop_path("/Data/file.bin/"), "Data/file.bin");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_directory_drop_preserves_the_root_and_relative_paths() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_root =
            std::env::temp_dir().join(format!("dzip-file-drop-{}-{unique}", std::process::id()));
        let dropped_root = test_root.join("Assets");
        let nested = dropped_root.join("Data");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(dropped_root.join("root.txt"), b"root").unwrap();
        std::fs::write(nested.join("nested.bin"), b"nested").unwrap();

        let mut files = Vec::new();
        collect_native_directory(&dropped_root, &mut files).unwrap();

        assert_eq!(
            files,
            vec![
                DroppedFile {
                    path: "Assets/Data/nested.bin".to_string(),
                    bytes: b"nested".to_vec(),
                },
                DroppedFile {
                    path: "Assets/root.txt".to_string(),
                    bytes: b"root".to_vec(),
                },
            ]
        );
        std::fs::remove_dir_all(test_root).unwrap();
    }
}
