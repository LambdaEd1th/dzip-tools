//! Optional, editable translations in the desktop package's assets.

use std::{
    collections::{BTreeSet, HashMap},
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use nsh_core::{i18n::Translations, platform::Locale};

type Messages = HashMap<String, String>;
type Catalogs = HashMap<String, Messages>;
const MAX_FILE_BYTES: u64 = 1024 * 1024;

pub(crate) fn load(builtin: &Translations) -> Catalogs {
    match std::env::current_exe() {
        Ok(exe) => locale_dir(&exe, std::env::consts::OS)
            .map(|dir| load_dir(&dir, builtin))
            .unwrap_or_default(),
        Err(error) => {
            log::warn!("Cannot locate the executable for external language files: {error}");
            Catalogs::new()
        }
    }
}

/// Select exactly one resource root. A portable copy must not pick up
/// translations from another installation (or the process working directory).
fn locale_dir(exe: &Path, os: &str) -> Option<PathBuf> {
    let bin = exe.parent()?;
    if os == "macos"
        && bin.ends_with("Contents/MacOS")
        && let Some(contents) = bin.parent()
        && contents
            .parent()?
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
    {
        return Some(contents.join("Resources/assets/locales"));
    }
    // Dioxus' Linux layout is relative to the installation prefix. Restrict
    // the lookup to our cargo-bundle product, even if lib/ has other apps.
    if os == "linux"
        && bin.ends_with("bin")
        && let Some(prefix) = bin.parent()
    {
        let installed = prefix.join("lib/dzip-gui/assets/locales");
        if installed.is_dir() {
            return Some(installed);
        }
    }
    Some(bin.join("assets/locales"))
}

fn load_dir(dir: &Path, builtin: &Translations) -> Catalogs {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Catalogs::new(),
        Err(e) => {
            log::warn!("Cannot list language directory {}: {e}", dir.display());
            return Catalogs::new();
        }
    };
    let mut files = Vec::new();
    for entry in entries {
        let file = entry.and_then(|entry| Ok(entry.file_type()?.is_file().then(|| entry.path())));
        match file {
            Ok(Some(path))
                if path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("toml")) =>
            {
                files.push(path);
            }
            Err(e) => log::warn!("Cannot inspect entry in {}: {e}", dir.display()),
            _ => {}
        }
    }
    // Stable order even if two filenames normalize to the same language.
    files.sort();
    let mut catalogs = Catalogs::new();
    for path in files {
        let Some(tag) = locale_tag(&path) else {
            log::warn!("Ignoring invalid language filename {}", path.display());
            continue;
        };
        let mut messages = match read_messages(&path) {
            Ok(messages) => messages,
            Err(e) => {
                log::warn!("Ignoring language file {}: {e}", path.display());
                continue;
            }
        };
        messages.retain(|key, text| {
            let Some(reference) = builtin.lookup("en", key) else {
                // This version of the app does not use this key.
                return false;
            };
            if parameters(&reference) == parameters(text) {
                return true;
            }
            log::warn!(
                "Ignoring {key:?} in {}: expected parameters from the built-in translation",
                path.display()
            );
            false
        });
        if !messages.is_empty() {
            catalogs.entry(tag).or_default().extend(messages);
        }
    }
    // Don't label a new, partial language pack "English" in the menu.
    for (tag, messages) in &mut catalogs {
        if !builtin.locales().contains(tag) {
            messages
                .entry("language-self".into())
                .or_insert_with(|| tag.clone());
        }
    }
    catalogs
}

fn locale_tag(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let locale = Locale::parse(stem)?;
    // Locale::parse also understands system settings like en_US.UTF-8 and
    // drops unsupported suffixes; never let a stray en.backup.toml override en.
    let tag = locale.as_str();
    stem.replace('_', "-")
        .eq_ignore_ascii_case(tag)
        .then(|| tag.to_owned())
}

fn read_messages(path: &Path) -> Result<Messages, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("file exceeds 1 MiB".into());
    }
    let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    toml::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.message().to_owned())
}

fn parameters(text: &str) -> Option<BTreeSet<&str>> {
    text.split("%{")
        .skip(1)
        .map(|part| {
            let (name, _) = part.split_once('}')?;
            (!name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
                .then_some(name)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_toml_files_round_trip_as_external_catalogs() {
        let json = include_str!(concat!(env!("OUT_DIR"), "/translations.json"));
        let builtin = Translations::from_json(json).unwrap();
        let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        assert_eq!(
            load_dir(&templates, &builtin),
            serde_json::from_str::<Catalogs>(json).unwrap()
        );
    }

    #[test]
    fn package_layout_selects_exactly_one_directory() {
        for (os, exe, dir) in [
            (
                "macos",
                "release/Dzip Archive.app/Contents/MacOS/dzip-gui",
                "release/Dzip Archive.app/Contents/Resources/assets/locales",
            ),
            ("linux", "release/dzip-gui", "release/assets/locales"),
            ("windows", "bin/dzip-gui.exe", "bin/assets/locales"),
            ("macos", "bin/dzip-gui", "bin/assets/locales"),
        ] {
            assert_eq!(
                locale_dir(Path::new(exe), os).as_deref(),
                Some(Path::new(dir))
            );
        }
    }

    #[test]
    fn linux_finds_its_own_installed_catalogs_then_portable_catalogs() {
        let root = temporary_dir("prefix");
        for layout in ["usr", "opt/dzip", "AppDir/usr"] {
            let prefix = root.join(layout);
            let exe = prefix.join("bin/dzip-gui");
            let portable = prefix.join("bin/assets/locales");
            let packaged = prefix.join("lib/dzip-gui/assets/locales");
            fs::create_dir_all(&portable).unwrap();
            fs::create_dir_all(prefix.join("lib/other-app/assets/locales")).unwrap();
            fs::create_dir_all(prefix.join("lib/dzip-gui/assets")).unwrap();
            assert_eq!(locale_dir(&exe, "linux").unwrap(), portable);
            fs::create_dir(&packaged).unwrap();
            assert_eq!(locale_dir(&exe, "linux").unwrap(), packaged);
            assert_eq!(
                locale_dir(&exe.with_extension("exe"), "windows").unwrap(),
                portable
            );
            assert_eq!(
                locale_dir(&prefix.join("tools/dzip-gui"), "linux").unwrap(),
                prefix.join("tools/assets/locales")
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn locale_filenames_allow_aliases_but_not_backup_suffixes() {
        assert_eq!(
            locale_tag(Path::new("ZH_hANT.toml")).as_deref(),
            Some("zh-Hant")
        );
        assert_eq!(locale_tag(Path::new("en.backup.toml")), None);
    }

    fn temporary_dir(label: &str) -> PathBuf {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("dzip-locales-{label}-{}-{id}", std::process::id()));
        fs::create_dir(&root).unwrap();
        root
    }

    #[test]
    fn restart_reloads_only_valid_external_entries_over_bundled_defaults() {
        let root = temporary_dir("edits");
        let dir = locale_dir(&root.join("dzip-gui"), "linux").unwrap();
        let builtin = Translations::from_json(r#"{"en":{"language-self":"English","title":"Title","welcome":"Hello %{name}","fallback":"Fallback"}}"#).unwrap();
        assert!(load_dir(&dir, &builtin).is_empty());
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("en.toml"),
            "title = 'Edited title'\nwelcome = 'Wrong %{who}'",
        )
        .unwrap();
        fs::write(dir.join("pt_BR.toml"), "\u{feff}welcome = 'Olá %{name}'").unwrap();
        fs::write(dir.join("en.backup.toml"), "fallback = 'Backup'").unwrap();
        fs::write(dir.join("de.toml"), "title = 2").unwrap();
        fs::write(dir.join("fr.toml"), [0xff]).unwrap();
        fs::write(dir.join("ja.toml"), vec![b' '; MAX_FILE_BYTES as usize + 1]).unwrap();
        let restored = builtin.extend(load_dir(&dir, &builtin));
        assert_eq!(restored.locales(), ["en", "pt-BR"]);
        assert_eq!(restored.translate("en", "title", &[]), "Edited title");
        assert_eq!(
            restored.translate("en", "welcome", &[("name", "Ada".to_string())]),
            "Hello Ada"
        );
        assert_eq!(
            restored.translate("pt-BR", "welcome", &[("name", "Ada".to_string())]),
            "Olá Ada"
        );
        assert_eq!(restored.translate("pt-BR", "language-self", &[]), "pt-BR");
        assert_eq!(restored.translate("pt-BR", "fallback", &[]), "Fallback");
        fs::write(dir.join("en.toml"), "title = 'Second edit'").unwrap();
        assert_eq!(
            builtin
                .extend(load_dir(&dir, &builtin))
                .translate("en", "title", &[]),
            "Second edit"
        );
        fs::remove_file(dir.join("en.toml")).unwrap();
        assert_eq!(
            builtin
                .extend(load_dir(&dir, &builtin))
                .translate("en", "title", &[]),
            "Title"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
