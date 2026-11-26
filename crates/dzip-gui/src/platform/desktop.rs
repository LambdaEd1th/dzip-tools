pub async fn save_bytes(file_name: &str, bytes: Vec<u8>) -> Result<String, String> {
    let Some(handle) = nsh_core::element::pick_save_path(
        nsh_core::file_dialog::SaveDialog::new().file_name(file_name),
    )
    .await?
    else {
        return Err("已取消保存".to_string());
    };
    let path = handle;
    crate::task::run_cpu_task(move || {
        std::fs::write(&path, bytes).map_err(|error| format!("保存失败：{error}"))?;
        Ok(path.display().to_string())
    })
    .await?
}

pub async fn save_archive_volumes(
    file_name: &str,
    mut volumes: Vec<(String, Vec<u8>)>,
) -> Result<String, String> {
    if volumes.len() == 1 {
        let Some((_, bytes)) = volumes.pop() else {
            return Err("归档没有生成可保存的数据".to_string());
        };
        return save_bytes(file_name, bytes).await;
    }
    let Some(folder) = nsh_core::element::pick_paths(
        nsh_core::file_dialog::OpenDialog::folders().title("选择分卷归档保存目录"),
    )
    .await?
    .and_then(|paths| paths.into_iter().next()) else {
        return Err("已取消保存".to_string());
    };
    let root = folder;
    crate::task::run_cpu_task(move || {
        let output = volumes
            .iter()
            .map(|(name, bytes)| dzip_workflow::NamedBytes {
                name: name.clone(),
                bytes: bytes.clone(),
            })
            .collect::<Vec<_>>();
        dzip_workflow::write_named_files(&root, &output).map_err(|error| error.to_string())?;
        Ok(format!(
            "已保存 {} 个分卷到 {}",
            volumes.len(),
            root.display()
        ))
    })
    .await?
}

pub async fn export_files(
    archive_name: &str,
    files: Vec<(String, Vec<u8>)>,
) -> Result<String, String> {
    let Some(folder) = nsh_core::element::pick_paths(
        nsh_core::file_dialog::OpenDialog::folders().title("选择解压目录"),
    )
    .await?
    .and_then(|paths| paths.into_iter().next()) else {
        return Err("已取消解压".to_string());
    };
    let root = folder;
    let archive_name = archive_name.to_string();
    crate::task::run_cpu_task(move || {
        let output = files
            .iter()
            .map(|(path, bytes)| dzip_workflow::ExtractedFile {
                path: path.clone(),
                bytes: bytes.clone(),
            })
            .collect::<Vec<_>>();
        dzip_workflow::write_extracted_files(&root, &output).map_err(|error| error.to_string())?;
        Ok(format!(
            "已从 {archive_name} 解压 {} 个文件到 {}",
            files.len(),
            root.display()
        ))
    })
    .await?
}
