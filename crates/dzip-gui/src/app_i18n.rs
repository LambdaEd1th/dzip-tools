//! Initialize NSH's surface dictionary and restore the saved UI language.

use nsh_core::{i18n::Translations, platform::Locale, prelude::*};

use crate::preferences;

pub(crate) fn initialize() {
    // Register sidecars before matching the preference: it may name a
    // language provided only by an external file.
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    add_translations(crate::desktop_locales::load(
        &global::<Translations>().get_untracked(),
    ));

    let translations = global::<Translations>().get_untracked();
    if let Some(locale) =
        preferences::read_locale().and_then(|tag| translations.pick([Locale::parse(&tag)?]))
    {
        after_answer(move || set_locale(Some(locale)));
    }
    // Without a supported preference, leave NSH following the device's
    // ordered list of languages, including future system-language changes.
}

/// Which catalog the surface displays (e.g. "fr" for the locale "fr-CA").
pub(crate) fn current_language() -> String {
    let dict = global::<Translations>().get();
    let requested = dict
        .pick(locale_settings().languages.iter())
        .unwrap_or_else(locale);
    catalog_tag(&dict, requested.as_str()).to_owned()
}

// NSH's pick preserves the requested locale (e.g. fr-CA). The selector
// needs the exact catalog tag from its list of items (e.g. fr or fr-CA).
fn catalog_tag<'a>(dict: &Translations, mut requested: &'a str) -> &'a str {
    loop {
        if dict.locales().iter().any(|tag| tag == requested) {
            return requested;
        }
        match requested.rsplit_once('-') {
            Some((parent, _)) => requested = parent,
            None => return "en",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    const JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/translations.json"));

    #[test]
    fn selector_uses_the_most_specific_installed_catalog() {
        let dict = Translations::from_json(r#"{"en":{}, "fr":{}, "zh":{}, "zh-Hant":{}}"#).unwrap();
        assert_eq!(catalog_tag(&dict, "zh-Hant"), "zh-Hant");
        assert_eq!(catalog_tag(&dict, "zh-Hant-TW"), "zh-Hant");
        assert_eq!(catalog_tag(&dict, "fr-CA"), "fr");
        assert_eq!(catalog_tag(&dict, "ja"), "en");
    }

    #[test]
    fn catalogs_match_in_keys_and_parameters() {
        type Catalogs = BTreeMap<String, BTreeMap<String, String>>;
        fn parameters(text: &str) -> BTreeSet<&str> {
            text.split("%{")
                .skip(1)
                .map(|p| p.split_once('}').unwrap().0)
                .collect()
        }
        let catalogs: Catalogs = serde_json::from_str(JSON).unwrap();
        let english = &catalogs["en"];
        for (locale, catalog) in &catalogs {
            assert!(catalog.keys().eq(english.keys()), "keys differ: {locale}");
            for (key, value) in catalog {
                assert_eq!(
                    parameters(value),
                    parameters(&english[key]),
                    "{locale}.{key}"
                );
            }
        }
    }

    #[test]
    fn native_labels_and_named_parameters() {
        let dict = Translations::from_json(JSON).unwrap();
        assert_eq!(dict.locales(), ["en", "es", "fr", "ru", "zh"]);
        for (tag, label) in [
            ("en", "English"),
            ("es", "Español"),
            ("fr", "Français"),
            ("ru", "Русский"),
            ("zh", "简体中文"),
        ] {
            assert_eq!(dict.translate(tag, "language-self", &[]), label);
        }
        assert_eq!(
            dict.translate("de", "opened-archive", &[("name", "demo.dz".into())]),
            "Opened demo.dz"
        );
    }

    #[test]
    fn native_matching_accepts_old_preferences_and_skips_unavailable_languages() {
        let dict = Translations::from_json(JSON).unwrap();
        let locales = ["de-DE", "zh_CN.UTF-8", "en-US"]
            .into_iter()
            .filter_map(Locale::parse);
        let chosen = dict.pick(locales).unwrap();
        assert_eq!(chosen.as_str(), "zh-CN");
        assert_eq!(
            dict.translate(chosen.as_str(), "language-self", &[]),
            "简体中文"
        );
        assert!(dict.pick(Locale::parse("de-DE")).is_none());
    }
}
