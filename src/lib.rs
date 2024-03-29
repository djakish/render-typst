use comemo::Prehashed;
use std::{collections::HashMap, sync::{OnceLock, RwLock}};
use typst::{diag::FileResult, eval::Tracer, foundations::{Bytes, Datetime}, model::Document, syntax::{FileId, Source, VirtualPath}, text::{Font, FontBook}, Library, World, LibraryBuilder};
use typst::foundations::{Smart, Str, Value};
use typst::foundations::Dict;
use typst_pdf;
use typst_svg;
use typst_render;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

static mut INSTANCE: OnceLock<WasmWorld> = OnceLock::new();

#[wasm_bindgen(module = "/js/utils.js")]
extern "C" {
    async fn fetchDataAsByteArray(fontUrl: &str) -> JsValue;
}

#[wasm_bindgen(start)]
fn start() {
    unsafe {
        INSTANCE.get_or_init(|| {
            let book = FontBook::new();
            let fonts = Vec::new();
            WasmWorld {
                library: Prehashed::new(LibraryBuilder::default().build()),
                book: Prehashed::new(book),
                fonts,
                main: FileId::new(None, VirtualPath::new("main.typ")),
                slots: RwLock::new(HashMap::new()),
            }
        })
    };
}

fn world() -> &'static mut WasmWorld {
    unsafe { INSTANCE.get_mut().unwrap() }
}

#[wasm_bindgen(js_name = addFont)]
pub async fn add_font(font_url: &str) -> Result<(), JsValue> {
    let font = fetchDataAsByteArray(font_url).await;
    let array = js_sys::Uint8Array::new(&font);
    let bytes: Vec<u8> = array.to_vec();

    let world = world();
    let buffer = Bytes::from(bytes);
    for font in Font::iter(buffer) {
        world.book.update(|book| book.push(font.info().clone()));
        world.fonts.push(font);
    }

    Ok(())
}

#[wasm_bindgen(js_name = addFile)]
pub async fn add_file(data_url: &str, name: &str) -> Result<(), JsValue> {
    let file = fetchDataAsByteArray(data_url).await;
    let array = js_sys::Uint8Array::new(&file);
    let bytes: Vec<u8> = array.to_vec();

    let world = world();
    let buffer = Bytes::from(bytes);

    let key = FileId::new(None, VirtualPath::new(name));
    let source = Source::detached(name);

    world.slots.write().unwrap().entry(key).or_insert_with(|| FileSlot {
        source: OnceLock::from(Ok(source.clone())),
        buffer: OnceLock::from(Ok(buffer.clone()))
    });

    Ok(())
}

#[wasm_bindgen(js_name = addSource)]
pub fn add_source(text: &str, name: &str) {
    let world = world();
    let file_id = FileId::new(None, VirtualPath::new(name));
    let source = Source::new(file_id, String::from(text));

    world.slots.write().unwrap().insert(file_id, FileSlot {
        source: OnceLock::from(Ok(source.clone())),
        buffer: OnceLock::from(Ok(Bytes::from(source.text().as_bytes().to_vec())))
    });
}

#[wasm_bindgen(js_name = setSource)]
pub fn set_source(text: &str) {
    add_source(text, "main.typ")
}

#[wasm_bindgen(js_name = setInputs)]
pub fn set_inputs(value: JsValue) {
    let inputs: HashMap<String, String> = serde_wasm_bindgen::from_value(value).unwrap();
    let mut dict = Dict::new();
    for (key, value) in inputs {
        dict.insert(Str::from(key), Value::Str(Str::from(value)));
    }
    let world = world();
    world.library = Prehashed::new(LibraryBuilder::default().with_inputs(dict).build());
}

fn compile() -> Document {
    let world = world();
    let mut tracer = Tracer::new();

    typst::compile(world, &mut tracer).unwrap()
}

#[wasm_bindgen(js_name = renderSvgMerged)]
pub fn render_svg_merged() -> String {
    let document = compile();

    typst_svg::svg_merged(&document, typst::layout::Abs::pt(5.0))
}

#[wasm_bindgen(js_name = renderSvg)]
pub fn render_svg(page: usize) -> String {
    let document = compile();

    typst_svg::svg(&document.pages[page].frame)
}

#[wasm_bindgen(js_name = renderPng)]
pub fn render_png(page: usize, pixel_per_pt: f32) -> Vec<u8> {
    let document = compile();

    let pixmap = typst_render::render(&document.pages[page].frame, pixel_per_pt, typst::visualize::Color::WHITE);
    
    pixmap.encode_png().unwrap()
}

#[wasm_bindgen(js_name = renderPdf)]
pub fn render_pdf() -> Vec<u8> {
    let document = compile();
    let world = world();

    typst_pdf::pdf(&document, Smart::Auto, world.today(Some(0)))
}

#[wasm_bindgen(js_name = pagesCount)]
pub fn pages_count() -> usize {
    let document = compile();

    document.pages.len()
}

pub struct WasmWorld {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
    slots: RwLock<HashMap<FileId, FileSlot>>,
}

#[derive(Clone)]
struct FileSlot {
    source: OnceLock<FileResult<Source>>,
    buffer: OnceLock<FileResult<Bytes>>,
}

impl World for WasmWorld {
    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    fn main(&self) -> Source {
        self.source(self.main).unwrap()
    }

    fn source(&self, _id: FileId) -> FileResult<Source> {
        let locked_slot = self.slots.read().unwrap();
        let slot = locked_slot.get(&_id).unwrap();
        slot.source.get().unwrap().clone()
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let locked_slot = self.slots.read().unwrap();
        let slot = locked_slot.get(&id).unwrap();
        let bytes = slot.buffer.get().unwrap();

        bytes.clone()
    }

    fn font(&self, index: usize) -> Option<Font> {
        Some(self.fonts[index].clone())
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        Some(Datetime::from_ymd(1970, 1, 1).unwrap())
    }
}
