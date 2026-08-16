//! Render Typst documents in the browser.
//!
//! The compiler talks to a [`World`] that lives entirely in memory: sources and
//! binary files are pushed in from JavaScript, packages from the `@preview`
//! namespace are downloaded on demand by [`prepare_packages`].

use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use typst::diag::{FileError, FileResult, SourceDiagnostic, Warned};
use typst::foundations::{Bytes, Datetime, Dict, Duration, Str, Value};
use typst::layout::Abs;
use typst::syntax::package::PackageSpec;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::{LazyHash, Scalar};
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;
use typst_render::RenderOptions;
use typst_svg::SvgOptions;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

/// Where `@preview` packages are downloaded from.
const PACKAGE_REGISTRY: &str = "https://packages.typst.org/preview";

/// The file every document is compiled from.
const MAIN: &str = "main.typ";

/// How many compilations a memoized result survives without being used.
const COMEMO_EVICT_MAX_AGE: usize = 10;

#[wasm_bindgen(module = "/js/utils.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn fetchDataAsByteArray(dataUrl: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    async fn fetchPackage(url: &str) -> Result<JsValue, JsValue>;
}

// ---------------------------------------------------------------------------
// Global world
// ---------------------------------------------------------------------------

static WORLD: OnceLock<RwLock<WasmWorld>> = OnceLock::new();

fn world() -> &'static RwLock<WasmWorld> {
    WORLD.get_or_init(|| RwLock::new(WasmWorld::new()))
}

/// Borrows the world for compilation. Never blocks: WASM is single-threaded.
fn read_world() -> RwLockReadGuard<'static, WasmWorld> {
    world().read().unwrap_or_else(PoisonError::into_inner)
}

/// Borrows the world for mutation.
fn write_world() -> RwLockWriteGuard<'static, WasmWorld> {
    world().write().unwrap_or_else(PoisonError::into_inner)
}

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

/// Downloads a font and registers all faces it contains.
#[wasm_bindgen(js_name = addFont)]
pub async fn add_font(font_url: &str) -> Result<(), JsValue> {
    let bytes = fetch_bytes(font_url).await?;

    let mut world = write_world();
    for font in Font::iter(bytes) {
        // Mutating through `LazyHash` resets its cached hash for us.
        world.book.push(font.info().clone());
        world.fonts.push(font);
    }

    Ok(())
}

/// Downloads a file and mounts it at `name`.
///
/// Works for anything the document reads: images, data files, and `.typ`
/// sources, which are parsed the first time they are imported.
#[wasm_bindgen(js_name = addFile)]
pub async fn add_file(data_url: &str, name: &str) -> Result<(), JsValue> {
    let bytes = fetch_bytes(data_url).await?;
    let id = project_file(name)?;

    write_world().insert_file(id, bytes);

    Ok(())
}

/// Mounts a Typst source file at `name`.
#[wasm_bindgen(js_name = addSource)]
pub fn add_source(text: &str, name: &str) -> Result<(), JsValue> {
    let id = project_file(name)?;

    write_world().insert_source(id, text.to_string());

    Ok(())
}

/// Mounts the entrypoint, `main.typ`.
#[wasm_bindgen(js_name = setSource)]
pub fn set_source(text: &str) -> Result<(), JsValue> {
    add_source(text, MAIN)
}

/// Sets the values readable from `sys.inputs`.
#[wasm_bindgen(js_name = setInputs)]
pub fn set_inputs(value: JsValue) -> Result<(), JsValue> {
    let inputs: HashMap<String, String> =
        serde_wasm_bindgen::from_value(value).map_err(|err| JsValue::from_str(&err.to_string()))?;

    let mut dict = Dict::new();
    for (key, value) in inputs {
        dict.insert(Str::from(key), Value::Str(Str::from(value)));
    }

    write_world().library = LazyHash::new(Library::builder().with_inputs(dict).build());

    Ok(())
}

// ---------------------------------------------------------------------------
// Packages
// ---------------------------------------------------------------------------

/// Mounts a package of your own from a `.tar`/`.tar.gz` at any URL.
///
/// `spec` is what the source will import it as, so a package mounted as
/// `@local/notes:1.0.0` is used with `#import "@local/notes:1.0.0"`. Any
/// namespace works; a package registered here is never downloaded again by
/// `preparePackages`. The archive needs the same layout as a Typst package: a
/// `typst.toml` at its root naming the entrypoint.
///
/// Whatever the package imports from `@preview` is downloaded along with it.
#[wasm_bindgen(js_name = addPackage)]
pub async fn add_package(spec: &str, tar_url: &str) -> Result<(), JsValue> {
    let spec = spec
        .parse::<PackageSpec>()
        .map_err(|err| JsValue::from_str(&format!("{spec}: {err}")))?;

    let tar = fetchPackage(tar_url).await.map(to_vec)?;

    {
        let mut world = write_world();
        untar(&tar, &spec, &mut world)?;
        world.packages.insert(spec);
    }

    prepare_packages().await
}

/// Downloads every `@preview` package the current sources import.
///
/// Call this after the sources are set and before rendering. Packages imported
/// by packages are picked up too, and nothing is downloaded twice.
#[wasm_bindgen(js_name = preparePackages)]
pub async fn prepare_packages() -> Result<(), JsValue> {
    // The world is only borrowed inside these blocks: a lock must never be held
    // across an `.await`.
    loop {
        let missing = read_world().missing_packages();
        if missing.is_empty() {
            return Ok(());
        }

        for spec in missing {
            let url = format!("{PACKAGE_REGISTRY}/{}-{}.tar.gz", spec.name, spec.version);
            let tar = fetchPackage(&url).await.map(to_vec)?;

            let mut world = write_world();
            untar(&tar, &spec, &mut world)?;
            world.packages.insert(spec);
        }
    }
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// Renders all pages into a single SVG, stacked vertically.
#[wasm_bindgen(js_name = renderSvgMerged)]
pub fn render_svg_merged() -> Result<String, JsValue> {
    let document = compile()?;

    Ok(typst_svg::svg_merged(
        &document,
        &SvgOptions::default(),
        Abs::pt(5.0),
    ))
}

/// Renders a single page into an SVG.
#[wasm_bindgen(js_name = renderSvg)]
pub fn render_svg(page: usize) -> Result<String, JsValue> {
    let document = compile()?;
    let page = nth_page(&document, page)?;

    Ok(typst_svg::svg(page, &SvgOptions::default()))
}

/// Renders a single page into a PNG at `pixel_per_pt` resolution.
#[wasm_bindgen(js_name = renderPng)]
pub fn render_png(page: usize, pixel_per_pt: f32) -> Result<Vec<u8>, JsValue> {
    let document = compile()?;
    let page = nth_page(&document, page)?;

    let options = RenderOptions {
        pixel_per_pt: Scalar::new(f64::from(pixel_per_pt)),
        ..RenderOptions::default()
    };

    typst_render::render(page, &options)
        .encode_png()
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Renders the whole document into a PDF.
#[wasm_bindgen(js_name = renderPdf)]
pub fn render_pdf() -> Result<Vec<u8>, JsValue> {
    let document = compile()?;

    typst_pdf::pdf(&document, &PdfOptions::default()).map_err(|diagnostics| diagnostics_to_js(&diagnostics))
}

/// The number of pages the document has.
#[wasm_bindgen(js_name = pagesCount)]
pub fn pages_count() -> Result<usize, JsValue> {
    Ok(compile()?.pages().len())
}

fn compile() -> Result<PagedDocument, JsValue> {
    // Bound the memoization cache; without this it grows for the whole session.
    comemo::evict(COMEMO_EVICT_MAX_AGE);

    let world = read_world();
    let Warned { output, .. } = typst::compile::<PagedDocument>(&*world);

    output.map_err(|diagnostics| diagnostics_to_js(&diagnostics))
}

fn nth_page(document: &PagedDocument, index: usize) -> Result<&typst_layout::Page, JsValue> {
    document
        .pages()
        .get(index)
        .ok_or_else(|| JsValue::from_str(&format!("page {index} is out of bounds")))
}

fn diagnostics_to_js(diagnostics: &[SourceDiagnostic]) -> JsValue {
    let message = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    JsValue::from_str(&message)
}

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

struct WasmWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
    /// Every file the compiler can see, source or binary.
    files: HashMap<FileId, Bytes>,
    /// Parsed sources, filled in on demand by [`World::source`].
    sources: RwLock<HashMap<FileId, Source>>,
    /// Packages that have been downloaded already.
    packages: HashSet<PackageSpec>,
}

impl WasmWorld {
    fn new() -> Self {
        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(FontBook::new()),
            fonts: Vec::new(),
            // A literal with no backslash and no `..`, so this cannot fail.
            main: project_file(MAIN).expect("main.typ is a valid path"),
            files: HashMap::new(),
            sources: RwLock::new(HashMap::new()),
            packages: HashSet::new(),
        }
    }

    fn insert_file(&mut self, id: FileId, bytes: Bytes) {
        self.files.insert(id, bytes);
        self.forget(id);
    }

    fn insert_source(&mut self, id: FileId, text: String) {
        self.files.insert(id, Bytes::from_string(text));
        self.forget(id);
    }

    /// Drops the parse of a file, so a replaced file is picked up.
    fn forget(&mut self, id: FileId) {
        self.sources
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id);
    }

    /// Package specs imported by known sources that are not downloaded yet.
    fn missing_packages(&self) -> Vec<PackageSpec> {
        let mut missing = Vec::new();

        for (id, bytes) in &self.files {
            if id.get().vpath().extension() != Some("typ") {
                continue;
            }

            let Ok(text) = bytes.as_str() else { continue };
            for spec in package_specs(text) {
                if !self.packages.contains(&spec) && !missing.contains(&spec) {
                    missing.push(spec);
                }
            }
        }

        missing
    }
}

impl World for WasmWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if let Some(source) = self
            .sources
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&id)
        {
            return Ok(source.clone());
        }

        let bytes = self.file(id)?;
        let text = bytes.as_str().map_err(FileError::from)?;
        let source = Source::new(id, text.to_string());

        self.sources
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, source.clone());

        Ok(source)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.get(&id).cloned().ok_or_else(|| {
            // Debug prints the package spec too, so the message says which
            // root the file was looked for in.
            FileError::NotFound(format!("{:?}", id.get()).into())
        })
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        Datetime::from_ymd(1970, 1, 1)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn fetch_bytes(url: &str) -> Result<Bytes, JsValue> {
    let data = fetchDataAsByteArray(url).await.map(to_vec)?;

    Ok(Bytes::new(data))
}

fn to_vec(value: JsValue) -> Vec<u8> {
    js_sys::Uint8Array::new(&value).to_vec()
}

/// The id of a file in the project root, reporting invalid paths to JS.
fn project_file(name: &str) -> Result<FileId, JsValue> {
    let vpath =
        VirtualPath::new(name).map_err(|err| JsValue::from_str(&format!("{name}: {err}")))?;

    Ok(RootedPath::new(VirtualRoot::Project, vpath).intern())
}


/// Finds every `@preview/name:x.y.z` mentioned in a source file.
///
/// A plain text scan: it can pick up a spec inside a comment or a string, which
/// only costs a download that nothing ends up importing.
fn package_specs(text: &str) -> Vec<PackageSpec> {
    let mut specs = Vec::new();

    for (start, _) in text.match_indices("@preview/") {
        let Some(rest) = text.get(start..) else { continue };
        let token: String = rest
            .chars()
            .take_while(|&c| c.is_ascii_alphanumeric() || matches!(c, '@' | '/' | ':' | '.' | '-' | '_'))
            .collect();

        if let Ok(spec) = token.parse::<PackageSpec>() {
            specs.push(spec);
        }
    }

    specs
}

/// Unpacks a tar archive into the world, under the package's root.
///
/// Only the parts of the format Typst's package bundles use are handled:
/// regular files, with the `ustar` prefix field taken into account.
fn untar(data: &[u8], spec: &PackageSpec, world: &mut WasmWorld) -> Result<(), JsValue> {
    const BLOCK: usize = 512;

    let malformed = || JsValue::from_str(&format!("package {spec} is malformed"));

    let mut offset = 0;
    while let Some(header) = data.get(offset..offset + BLOCK) {
        offset += BLOCK;

        // Two zeroed blocks mark the end of the archive.
        if header.iter().all(|&byte| byte == 0) {
            break;
        }

        let name = tar_field(header, 0, 100).ok_or_else(malformed)?;
        let size = tar_size(header).ok_or_else(malformed)?;
        let kind = header.get(156).copied().ok_or_else(malformed)?;

        let content = data.get(offset..offset + size).ok_or_else(malformed)?;
        offset += size.div_ceil(BLOCK) * BLOCK;

        // Regular files only; directories and metadata entries carry no data we
        // need, since the paths are complete on their own.
        if kind != b'0' && kind != 0 {
            continue;
        }

        // Only POSIX ustar archives keep a path prefix at 345. GNU archives
        // put timestamps there, which would turn into a bogus directory.
        let posix = header.get(257..263) == Some(b"ustar\0");
        let prefix = match posix {
            true => tar_field(header, 345, 155).unwrap_or_default(),
            false => String::new(),
        };
        let path = match prefix.is_empty() {
            true => name,
            false => format!("{prefix}/{name}"),
        };

        let Ok(vpath) = VirtualPath::new(path.trim_start_matches("./")) else {
            continue;
        };

        let id = RootedPath::new(VirtualRoot::Package(spec.clone()), vpath).intern();
        world.insert_file(id, Bytes::new(content.to_vec()));
    }

    Ok(())
}

/// Reads a NUL-padded string field out of a tar header.
fn tar_field(header: &[u8], start: usize, len: usize) -> Option<String> {
    let field = header.get(start..start + len)?;
    let end = field.iter().position(|&byte| byte == 0).unwrap_or(len);

    Some(String::from_utf8_lossy(field.get(..end)?).into_owned())
}

/// Reads the octal size field out of a tar header.
fn tar_size(header: &[u8]) -> Option<usize> {
    let field = tar_field(header, 124, 12)?;

    usize::from_str_radix(field.trim(), 8).ok()
}
