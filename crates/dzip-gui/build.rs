use std::{env, fs, path::PathBuf};

fn main() {
    // The NSH CLI configures its generated Root automatically. The desktop
    // binary launched by Cargo needs the same dictionary on its own Root.
    println!("cargo:rerun-if-changed=locales");
    let app = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let translations = nsh_i18n::read_translations(&app).expect("invalid locales/ catalog");
    assert!(!translations.is_empty(), "missing locales/ catalog");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(
        out.join("translations.json"),
        nsh_i18n::translations_json(&translations),
    )
    .expect("could not write the NSH catalog");
}
