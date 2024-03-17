use comemo::Prehashed;
use std::{collections::HashMap, iter::Once, sync::{OnceLock, RwLock}};
use typst::{
    diag::FileResult, eval::Tracer, foundations::{Bytes, Datetime}, model::Document, syntax::{FileId, Source, VirtualPath}, text::{Font, FontBook}, Library, World
};
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
                library: Prehashed::new(Library::build()),
                book: Prehashed::new(book),
                fonts,
                source: Source::detached("main.typ"),
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

#[wasm_bindgen(js_name = setSource)]
pub fn set_source(text: &str) {
    let world = world();
    world.source.replace(text);
}

fn compile() -> Document {
    let world = world();
    let mut tracer = Tracer::new();

    typst::compile(world, &mut tracer).unwrap()
}

#[wasm_bindgen(js_name = renderSvgMerged)]
pub fn render_svg_merged() -> String {
    let document = compile();

    typst_svg::svg_merged(&document.pages, typst::layout::Abs::pt(5.0))
}

#[wasm_bindgen(js_name = renderSvg)]
pub fn render_svg(page: usize) -> String {
    let document = compile();

    typst_svg::svg(&document.pages[page])
}

#[wasm_bindgen(js_name = renderPng)]
pub fn render_png(page: usize, pixel_per_pt: f32) -> Vec<u8> {
    let document = compile();

    let pixmap = typst_render::render(&document.pages[page], pixel_per_pt, typst::visualize::Color::WHITE);
    
    pixmap.encode_png().unwrap()
}

#[wasm_bindgen(js_name = renderPdf)]
pub fn render_pdf() -> Vec<u8> {
    let document = compile();
    let world = world();

    typst_pdf::pdf(&document, Some(""), world.today(Some(0)))
}

pub struct WasmWorld {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<Font>,
    source: Source,
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
        self.source.clone()
    }

    fn source(&self, _id: FileId) -> FileResult<Source> {
        unimplemented!()
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let state = self.slots.read().unwrap();
        let data = state.get(&id).unwrap();
        let byte = data.buffer.get().unwrap();

        byte.clone()
    }

    fn font(&self, index: usize) -> Option<Font> {
        Some(self.fonts[index].clone())
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        Some(Datetime::from_ymd(1970, 1, 1).unwrap())
    }
}
