//! Regression cases retained from the original archive browser.
use crate::{
    archive_input::*,
    browser::*,
    file_drop,
    model::{CompressionChoice, DraftFile, EntryView},
};
use std::sync::Arc;
fn entry(id: usize, path: &str, size: u64) -> EntryView {
    let name = path.rsplit('/').next().unwrap_or(path).to_string();
    let folder = path
        .rsplit_once('/')
        .map(|(folder, _)| folder.to_string())
        .unwrap_or_else(|| "根目录".to_string());
    EntryView {
        id,
        path: path.to_string(),
        name,
        folder,
        size,
        packed_size: Some(size / 2),
        compression: CompressionChoice::Dz,
        volume: 0,
        chunks: 1,
    }
}

fn draft(id: u64, path: &str, size: usize) -> DraftFile {
    DraftFile {
        id,
        path: path.to_string(),
        bytes: Arc::from(vec![0u8; size]),
        compression: CompressionChoice::Dz,
        volume: 0,
        preserved_segments: None,
    }
}

#[test]
fn browser_listing_builds_virtual_folders_and_searches_globally() {
    let entries = vec![
        entry(0, "readme.txt", 20),
        entry(1, "Data/config.json", 40),
        entry(2, "Data/Images/logo.png", 60),
        entry(3, "Sounds/theme.ogg", 80),
    ];

    let (root_folders, root_files) = build_browser_listing(&entries, "", "");
    assert_eq!(
        root_folders
            .iter()
            .map(|folder| folder.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Data", "Sounds"]
    );
    assert_eq!(root_folders[0].file_count, 2);
    assert_eq!(root_folders[0].size, 100);
    assert_eq!(root_folders[0].packed_size, Some(50));
    assert_eq!(root_folders[0].volume, Some(0));
    assert_eq!(root_files[0].path, "readme.txt");

    let mut split_entries = entries.clone();
    split_entries[2].volume = 1;
    let (split_folders, _) = build_browser_listing(&split_entries, "", "");
    assert_eq!(split_folders[0].volume, None);

    let (data_folders, data_files) = build_browser_listing(&entries, "Data", "");
    assert_eq!(data_folders[0].path, "Data/Images");
    assert_eq!(data_files[0].path, "Data/config.json");

    let (search_folders, search_files) = build_browser_listing(&entries, "Data", "theme");
    assert!(search_folders.is_empty());
    assert_eq!(search_files[0].path, "Sounds/theme.ogg");

    assert_eq!(entry_ids_in_directory(&entries, ""), vec![0, 1, 2, 3]);
    assert_eq!(entry_ids_in_directory(&entries, "Data"), vec![1, 2]);
    assert_eq!(entry_ids_in_directory(&entries, "Data/Images"), vec![2]);
    assert!(entry_ids_in_directory(&entries, "Missing").is_empty());
}

#[test]
fn draft_browser_listing_navigates_and_preserves_archive_paths() {
    let files = vec![
        draft(1, "readme.txt", 20),
        draft(2, "Data/config.json", 40),
        draft(3, "Data/Images/logo.png", 60),
        draft(4, "Sounds/theme.ogg", 80),
    ];

    let (root_folders, root_files) = build_draft_browser_listing(&files, "");
    assert_eq!(
        root_folders
            .iter()
            .map(|folder| folder.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Data", "Sounds"]
    );
    assert_eq!(root_folders[0].file_count, 2);
    assert_eq!(root_folders[0].size, 100);
    assert_eq!(root_folders[0].packed_size, None);
    assert_eq!(root_folders[0].volume, Some(0));
    assert_eq!(root_files[0].path, "readme.txt");

    let mut split_files = files.clone();
    split_files[2].volume = 1;
    let (split_folders, _) = build_draft_browser_listing(&split_files, "");
    assert_eq!(split_folders[0].volume, None);

    let (data_folders, data_files) = build_draft_browser_listing(&files, "Data");
    assert_eq!(data_folders[0].path, "Data/Images");
    assert_eq!(data_files[0].path, "Data/config.json");

    assert_eq!(
        browser_breadcrumbs("Data/Images")
            .iter()
            .map(|crumb| crumb.path.as_str())
            .collect::<Vec<_>>(),
        vec!["Data", "Data/Images"]
    );
    assert_eq!(parent_browser_directory("Data/Images"), "Data");
    assert_eq!(
        replace_draft_file_name("Data/config.json", "settings.json"),
        "Data/settings.json"
    );
    assert_eq!(
        join_archive_path("Data/Images", "icons/logo.png"),
        "Data/Images/icons/logo.png"
    );
}

#[test]
fn removing_a_draft_folder_removes_all_descendants_only() {
    let mut files = vec![
        draft(1, "Data/config.json", 40),
        draft(2, "Data/Images/logo.png", 60),
        draft(3, "Database/index.bin", 20),
        draft(4, "readme.txt", 10),
    ];

    remove_draft_folder(&mut files, "Data");

    assert_eq!(
        files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        vec!["Database/index.bin", "readme.txt"]
    );
}

#[test]
fn selected_extraction_removes_the_current_parent_path() {
    let files = vec![
        ("Data/Images/UI/icon.png".to_string(), vec![1]),
        ("Data/Images/UI/logo.png".to_string(), vec![2]),
    ];

    let rebased = rebase_extracted_files(files, Some("Data/Images"));

    assert_eq!(
        rebased
            .iter()
            .map(|(path, _)| path.as_str())
            .collect::<Vec<_>>(),
        vec!["UI/icon.png", "UI/logo.png"]
    );
}

#[test]
fn extraction_keeps_full_paths_for_root_or_mixed_selections() {
    let files = vec![
        ("Data/config.json".to_string(), vec![1]),
        ("Sounds/theme.ogg".to_string(), vec![2]),
    ];

    assert_eq!(rebase_extracted_files(files.clone(), None), files);
    assert_eq!(rebase_extracted_files(files.clone(), Some("Data")), files);
}

#[test]
fn dropped_archive_finds_the_main_file_and_sorted_volumes() {
    let files = vec![
        file_drop::DroppedFile {
            path: "release/game.002".to_string(),
            bytes: vec![2],
        },
        file_drop::DroppedFile {
            path: "release/readme.txt".to_string(),
            bytes: vec![9],
        },
        file_drop::DroppedFile {
            path: "release/game.DZ".to_string(),
            bytes: vec![0],
        },
        file_drop::DroppedFile {
            path: "release/game.001".to_string(),
            bytes: vec![1],
        },
    ];

    let (main_name, main_bytes, auxiliary) = prepare_dropped_archive(files).unwrap();

    assert_eq!(main_name, "game.DZ");
    assert_eq!(main_bytes, vec![0]);
    assert_eq!(
        auxiliary,
        vec![
            ("game.001".to_string(), vec![1]),
            ("game.002".to_string(), vec![2]),
        ]
    );
}

#[test]
fn dropped_archive_requires_exactly_one_main_file() {
    let volume_only = vec![file_drop::DroppedFile {
        path: "game.001".to_string(),
        bytes: vec![1],
    }];
    assert_eq!(
        prepare_dropped_archive(volume_only),
        Err(DroppedArchiveError::NoMainArchive)
    );

    let multiple = vec![
        file_drop::DroppedFile {
            path: "one.dz".to_string(),
            bytes: vec![1],
        },
        file_drop::DroppedFile {
            path: "two.dzip".to_string(),
            bytes: vec![2],
        },
    ];
    assert_eq!(
        prepare_dropped_archive(multiple),
        Err(DroppedArchiveError::MultipleMainArchives)
    );
}
