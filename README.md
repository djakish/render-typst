
SVG/PDF render some typst text, only got it working with vite.

# Usage

```ts
import init, { addFont, setSource, renderSvgMerged } from '@djakish/render-typst'
import lin_font_r from '../assets/fonts/LinLibertine_R.ttf'

// Load a font
await addFont(lin_font_r)

// Set source to the wasm 
setSource(`#text("Hello world!",fill: red)`);

// Get rendered SVG
let doc = renderSvgMerged()
```

# Vite dependencies for wasm 

With vite you would need [vite-plugin-wasm](https://www.npmjs.com/package/vite-plugin-wasm) and [vite-plugin-top-level-await](https://www.npmjs.com/package/vite-plugin-top-level-await).


# Building 

You need wasm-pack and rust, and dependecies for them.

```sh
wasm-pack build --target bundler 
```