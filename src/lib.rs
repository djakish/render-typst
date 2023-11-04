use comemo::Prehashed;
use std::sync::OnceLock;
use typst::{
    diag::FileResult,
    doc::Document,
    eval::{Bytes, Datetime, Library, Tracer},
    font::{Font, FontBook},
    syntax::{FileId, Source},
    World,
};
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
                library: Prehashed::new(typst_library::build()),
                book: Prehashed::new(book),
                fonts,
                source: Source::detached(""),
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

    typst::export::svg_merged(&document.pages, typst::geom::Abs::pt(5.0))
}

#[wasm_bindgen(js_name = renderSvg)]
pub fn render_svg(page: usize) -> String {
    let document = compile();

    typst::export::svg(&document.pages[page])
}

#[wasm_bindgen(js_name = renderPng)]
pub fn render_png(page: usize, pixel_per_pt: f32) -> Vec<u8> {
    let document = compile();

    let pixmap = typst::export::render(&document.pages[page], pixel_per_pt, typst::geom::Color::WHITE);
    
    pixmap.encode_png().unwrap()
}

#[wasm_bindgen(js_name = renderPdf)]
pub fn render_pdf() -> Vec<u8> {
    let document = compile();
    let world = world();

    typst::export::pdf(&document, Some(""), world.today(Some(0)))
}



pub struct WasmWorld {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<Font>,
    source: Source,
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

    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        unimplemented!()
    }

    fn font(&self, index: usize) -> Option<Font> {
        Some(self.fonts[index].clone())
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        Some(Datetime::from_ymd(1970, 1, 1).unwrap())
    }
}
