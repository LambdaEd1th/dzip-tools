# dzip-tools

[![Rust CI](https://github.com/LambdaEd1th/dzip-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/LambdaEd1th/dzip-tools/actions/workflows/ci.yml)

Command-line and graphical applications for reading, extracting, creating, and
inspecting Dzip archives. This workspace contains the shared DCL and
application-workflow layers, the `dzip-cli` command-line application, and a
Native Shell desktop/Web archive manager. The reusable library is maintained in
[dzip-archive](https://github.com/LambdaEd1th/dzip-archive) and pinned here as a Git
submodule.

The implementation is compatible with `dzip.exe`, including split volumes,
native DZ compression and COMBUF references, BZip2, LZMA1, and the original
program's unusual truncated-gzip/DEFLATE framing. All four codec engines are
integrated into the `dzip` crate with no C, FFI, or external compression
library. Standard-codec output need not match the original encoder byte for
byte; native DZ and COMBUF output is tested for byte identity.

The integrated codecs share typed options and errors, allocation-reusing
internals, and configurable input, output, and workspace limits.
`ReadOptions::limits` propagates those ceilings into archive decoding.

## Core library

Add the public crate directly from GitHub (it is not published on crates.io):

```toml
[dependencies]
dzip = { git = "https://github.com/LambdaEd1th/dzip-archive.git", tag = "v0.5.1" }
```

Open, inspect, and extract an archive:

```rust,no_run
use dzip::{Archive, ExtractOptions};

fn main() -> dzip::Result<()> {
    let mut archive = Archive::open_path("game.dz")?;

    for entry in archive.entries() {
        println!(
            "{}: {} bytes, {}",
            entry.path().display(),
            entry.decompressed_size(),
            entry.compression(),
        );
    }

    let config = archive.read_entry_by_path("Data/config.bin")?;
    println!("config contains {} bytes", config.len());

    archive.extract_to("output", ExtractOptions::default())?;
    Ok(())
}
```

Create a deterministic archive:

```rust,no_run
use dzip::{ArchiveBuilder, Compression, EntryOptions};

fn main() -> dzip::Result<()> {
    let mut builder = ArchiveBuilder::new();
    builder.add_path(
        "Data/config.bin",
        "input/config.bin",
        EntryOptions::new().compression(Compression::Dz),
    )?;
    builder.add_bytes(
        "readme.txt",
        b"hello from dzip".to_vec(),
        EntryOptions::new().compression(Compression::Zlib),
    )?;
    builder.write_to_path("game.dz")?;
    Ok(())
}
```

For split or embedded archives, use `VolumeSource`/`VolumeSink`,
`MemoryVolumeSource`, and `MemoryVolumeSink`. The lower-level format reader and
writer remain available for inspection and reverse-engineering tools.

Backends that need the stored auxiliary names before constructing their volume
provider can use `ArchivePreparation`. It owns the already-parsed main reader,
exposes lossless metadata, and then opens the semantic `Archive` without parsing
the header a second time.

For lossless metadata inspection, `RawArchive` preserves nul-terminated name
bytes and the chunk records exactly as stored. The semantic `ArchiveIndex`
separates those records from `resolved_chunks()`, whose physical lengths are
derived from volume layout for decoding. Each `Entry` also exposes `segments()`
so multi-chunk files retain their per-segment codec, volume, and decoded range.
This keeps host path validation and UTF-8 conversion out of format-level tools.

`ArchiveImage` retains the main file and every auxiliary volume byte-for-byte.
Use it when an archive must be inspected, transported, or rewritten without
decoding and rebuilding its payloads. `ArchiveImage::write_to_path` preserves
all retained bytes and uses the original auxiliary-volume names.

### Compatibility and safety

Archives always use the original dzip.exe compatibility behavior: the writer
reproduces its known physical-length quirks, while the reader repairs those
fields from the physical chunk layout. `Compression::Zero` follows the original
behavior and represents the requested length as zero bytes regardless of the
input contents.

Legacy DCL files may combine multiple storage flags. The library preserves the
complete flag word and applies dzip.exe's registered-coder priority. It also
preserves the original edge case where a retained `DZ` bit makes the reader
expect archive-wide DZ settings even when another encoder won; such an archive
is rejected by both readers. New code should select exactly one storage method
unless reproducing a legacy DCL file is required.

Auxiliary volumes are resolved lazily, matching dzip.exe: listing an archive
does not require every named volume to exist, while reading an entry reports a
missing volume only if that entry uses it. Archive paths treat both `/` and `\`
as separators on every host, use ASCII case-insensitive Windows comparison
rules, and are stored with Windows-style `\` separators.
DCL syntax is detected from its contents, so the root configuration does not
need a `.dcl` filename extension.

High-level extraction rejects absolute paths, parent traversal, existing
symlink parents, and symlink output targets. `ReadLimits` bounds metadata and
decompressed output when processing untrusted archives.

### Features

- `encode` and `decode` enable the corresponding public workflows.
- `bzip`, `dz`, `zlib`, and `lzma` select codec engines independently.
- `all-codecs` enables all four engines and is enabled by default.
- `parallel` uses bounded parallel batches for independent chunk encoding;
  archive-scoped DZ input is still processed together to preserve COMBUF
  behavior.
- `serde` adds serialization support to public configuration enums.

Archive metadata, `RangeSettings`, `Compression`, and builder options remain
available without codec features. Attempting to read or write a chunk whose
engine is disabled returns `CodecError::Unavailable`.

## CLI

Build from source:

```bash
git clone --recurse-submodules https://github.com/LambdaEd1th/dzip-tools.git
cd dzip-tools
cargo build --release -p dzip-cli
```

If the repository was cloned without submodules, initialize the core library
before building:

```bash
git submodule update --init --recursive
```

The executable is written to `target/release/dzip-cli`.

```bash
dzip-cli list game_data.dz
dzip-cli extract game_data.dz --output extracted
dzip-cli build build/game.dcl --output rebuilt \
  --command "align 2048"
dzip-cli create game_data.dz Data/config.bin Images/logo.png \
  --dir assets --type zlib --output rebuilt
```

The CLI is defined entirely with Clap derive. `build` accepts the legacy
`dzip.exe` `.dcl` syntax, while `create` provides a typed direct
creation workflow without the original executable's order-dependent argument
state. `extract` writes files only; it does not generate a repack manifest.

The DCL
frontend supports quoted paths, `master` includes, the original global
`basedir` search order, combined file flags, byte/percentage ranges, DZ option
blocks, and repeatable `-c`/`--command` overrides. Compatibility parsing keeps
the original case-insensitive and permissive numeric behavior; unsafe archive
entry traversal and recursive include cycles are still rejected.

## GUI

`dzip-gui` is a Native Shell archive manager that shares one responsive Rust interface
between native desktop builds and the browser. It can open and inspect archives,
search and select entries, extract files, and create new archives with DZ, Zlib,
Bzip, LZMA, copy, or zero-fill encoding.

Start the desktop app from the workspace root. Linux builds require GTK 4.14 or newer and its development headers.

```bash
cargo run -p dzip-gui --features desktop
```

For the browser, install the [Native Shell CLI 0.7.1](https://github.com/longbridge/nsh/releases/tag/v0.7.1)
for your host and put its `nsh` binary on your PATH. The Web build also uses Clang,
LLVM's archiver and the matching wasm-bindgen CLI.

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130 --locked
```

Build the Web app with its archive worker and serve it at `http://localhost:8080`.

```bash
bash scripts/build-web.sh
python3 -m http.server 8080 --directory 'target/nsh-web/Dzip Archive'
```

The desktop build saves archives with the native file dialog and extracts to a
chosen directory. Both frontends use the same typed build plans and stateful
archive-session service. Desktop builds keep one archive backend thread alive;
the browser keeps one Web Worker alive. After opening an archive, subsequent
operations send only its session ID and entry IDs instead of copying every
volume through the UI. Codec work remains parallel on supported desktop builds.

The web build processes files entirely in WebAssembly, downloads created `.dz`
archives directly, and bundles extracted entries into a browser-friendly ZIP
download. Select the main `.dz` file and its auxiliary volumes together when
opening a split archive.

The window, archive browser, editor, dialogs, icons, and responsive layout
are authored in Rust under `crates/dzip-gui/src`. Both targets share those
views; the Web build uses generated browser glue and a small worker bootstrap.
No HTML/CSS copy of the application is needed for the desktop build.

**Desktop translations.** Edit a language's TOML file in the resource directory
for your build, then quit and reopen the GUI. No rebuild or import is needed.
Desktop packages include the five default catalogs:

| Desktop build | Where to edit (Chinese example) |
| --- | --- |
| macOS app | `Dzip Archive.app/Contents/Resources/assets/locales/zh.toml` (Finder → Show Package Contents) |
| Windows ZIP | `assets/locales/zh.toml` beside `dzip-gui.exe` |
| Linux portable ZIP or a standalone Cargo binary | `assets/locales/zh.toml` beside the GUI executable |
| Debian installation | `/usr/lib/dzip-gui/assets/locales/zh.toml` |

On Linux, when the GUI is in `<prefix>/bin/`, it first looks for
`<prefix>/lib/dzip-gui/assets/locales/`. If that directory is absent, it uses
`assets/locales/` beside the executable. This also works with an extracted
Debian package, without installing it system-wide.

The working directory is never searched. If you used the earlier layout, copy
your customized TOML files from the old `locales/` folder to the location above;
the old folder is no longer read. Editing resources in a signed macOS app
invalidates its signature, and a package update can replace your edits. Keep a
copy of customized files.

A file can override just one key; omitted keys fall back to the bundled
translations. For example, a `zh.toml` containing only
`page-archive-manager = "我的归档"` changes the archive page title when
Chinese is selected. Keep named placeholders such as `%{name}` intact:
entries with mismatched placeholders are skipped. To add a language, use
its tag as the filename (e.g. `ja.toml`) and set `language-self = "日本語"`
for its name in Settings. To undo an override, remove the corresponding
file or key and restart. Invalid TOML files are ignored.

For example, after building a default Cargo debug binary, copy a catalog into
its resource directory, edit the copy, then rerun `target/debug/dzip-gui`:

```bash
mkdir -p target/debug/assets/locales
cp -i crates/dzip-gui/locales/zh.toml target/debug/assets/locales/
```

The sources for the built-in translations are in `crates/dzip-gui/locales/`.
Web uses only these bundled translations. Both targets follow the device's
preferred languages unless a language was selected in Settings.

**GUI logs.** Native Shell sends desktop diagnostics to the platform logger and
rotates `app.log` under the app's data directory (`dev.lambdaedith.dzip/logs/`).
Use `RUST_LOG` to adjust the desktop filter. In the browser, NSH uses the
developer console. On both targets, Settings → Logs shows the latest 300 GUI
records from the current session. Export saves that view; Clear doesn't erase
desktop log files or the browser console.

For a browser round-trip test, serve the bundle as above and run
`node scripts/smoke-web.cjs http://localhost:8080/` with Playwright available
to Node (see the script header). It imports a fixture through the file chooser,
edits and splits it, then checks the exported files against the original bytes.
Pass `--i18n` after the URL to run just the language-switching and preference tests.
Use `--logs` to test notices and session logs, or `--controls` to test keyboard
selection, nested dialogs and focus restoration.

## Workspace

```text
crates/
├── dzip-cli/             CLI and DCL compatibility frontend
├── dzip-dcl/             Reusable dzip.exe-compatible DCL parser
├── dzip-workflow/        Shared plans, lazy sessions, typed protocol, and exports
├── dzip-gui/             Native Shell views, file browser, and input state
└── dzip-worker/          Browser Worker runtime for archive operations
vendor/
└── dzip/                 Pinned dzip-archive submodule with integrated codecs
```

The workspace and the core library use the Rust 2024 edition. Core API,
codec, fixture, and compatibility-test changes belong in the dzip-archive
repository; update this repository's submodule pointer only after those changes
have passed its CI.

### Releases

This application workspace is distributed through GitHub Releases rather than
crates.io. Application crates inherit the workspace version, and a release tag
such as `v0.5.0` must match `dzip-cli` and the GUI manifest before the release workflow builds
artifacts. The core library has its own version and release tags.

## Replacing a `.dz` file in an Android APK

Only modify an APK that you own or are authorized to redistribute. An APK is a
ZIP archive, but changing any entry invalidates its existing signature. The
replacement must therefore happen before the final alignment and signing
steps.

The example below uses Bash, 7-Zip, and the Android SDK Build Tools. First
find the exact archive path and validate the replacement Dzip file:

```bash
7z l ./original.apk | grep -Ei '\.dz([[:space:]]|$)'

./target/release/dzip-cli list ./replacement/PvZ.dz
./target/release/dzip-cli extract ./replacement/PvZ.dz \
  --output ./replacement-check
```

Mirror the exact path reported by 7-Zip under a staging directory. Paths and
file names inside both APK and Dzip archives are case-sensitive on Android. In
this example the APK entry is `assets/PvZ.dz`:

```bash
cp ./original.apk ./app-unsigned-unaligned.apk
mkdir -p ./apk-payload/assets
cp ./replacement/PvZ.dz ./apk-payload/assets/PvZ.dz

(
  cd ./apk-payload
  7z u -tzip -mx=0 ../app-unsigned-unaligned.apk assets/PvZ.dz
)
```

The `-mx=0` example keeps the replacement APK entry uncompressed (`Store`), as
is common for large game assets. Check the original entry with `7z l -slt` and
preserve its method when required; omit `-mx=0` if the original entry uses
`Deflate`. If the Dzip archive has auxiliary volumes, mirror and replace every
volume in the same update.

Finally, align, sign, and verify the result. `zipalign` must run before
`apksigner`:

```bash
zipalign -P 16 -f -v 4 \
  ./app-unsigned-unaligned.apk ./app-aligned.apk
apksigner sign --ks ./release.jks \
  --out ./app-patched.apk ./app-aligned.apk

zipalign -c -P 16 -v 4 ./app-patched.apk
apksigner verify --verbose --print-certs ./app-patched.apk
```

See the Android documentation for
[`zipalign`](https://developer.android.com/tools/zipalign) and
[`apksigner`](https://developer.android.com/tools/apksigner). To install the
patched APK as an update, it must be signed with the same certificate as the
installed application. A different certificate requires uninstalling the old
application first, which normally removes its local data. Some applications
also perform their own integrity or certificate checks.

`dzip-cli extract` intentionally writes files only and does not preserve a
lossless repack manifest. Recreating an archive from that directory may change
entry order, flags, ranges, compression methods, COMBUF layout, or split-volume
layout. Use a reviewed DCL build configuration when those details matter,
and test the finished APK on a clean installation before distribution.

## License

This repository is distributed under AGPL-3.0-or-later. The dzip-archive
submodule is licensed separately under the same license; the fixed BZip2
legacy-randomization table retains its upstream notice in
`vendor/dzip/LICENSES/BZIP2.txt`.
