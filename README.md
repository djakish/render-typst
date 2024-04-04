
SVG/PDF render some typst text, only got it working with vite, and kind of with webpack.

# Usage

```ts
import init, { addFont, addSource, renderSvgMerged } from '@djakish/render-typst'
import lin_font_r from '../assets/fonts/LinLibertine_R.ttf'

// Load a font
await addFont(lin_font_r)

// Set input values
setInputs({
    "name": "world",
})

// Set the main source file
addSource(`#text([Hello #sys.inputs.name!],fill: red)`, "main.typ")

// Get rendered SVG
let doc = renderSvgMerged()
```

# Vite dependencies for wasm 

With vite you would need [vite-plugin-wasm](https://www.npmjs.com/package/vite-plugin-wasm) and [vite-plugin-top-level-await](https://www.npmjs.com/package/vite-plugin-top-level-await).


# Setting up with webpack 

Next config that I got to work.
```js
const nextConfig = {
  reactStrictMode: true,
  webpack: (config, { buildId, dev, isServer, defaultLoaders, webpack }) => {
    // For wasm
    config.externals.experiments = {
      asyncWebAssembly: true,
      importAsync: true,
      layers: true,
    }

    config.experiments = {
      asyncWebAssembly: true,
      layers: true,
    }

    config.module?.rules?.push({
      test: /\.bin$/i,
      type: 'asset/resource',
      generator: {
        filename: 'assets/[hash][ext][query]',
      },
    });
    return config
  }
}
```

Component that worked

```jsx
<button onClick={async (e) => {
    const typst = (await import("@djakish/render-typst"));
    await typst.addFont("/LinLibertine_R.ttf")
    typst.addSource(`#text("Hello world!",fill: red)`, "main.typ");
    let doc = typst.renderSvgMerged()
    let preview = document.querySelector<HTMLDivElement('#preview')!;
    preview.innerHTML = doc
    }}>Render</button>
 <div id='preview'></div>
```

# Building 

You need wasm-pack and rust, and dependencies for them.

```sh
wasm-pack build --target bundler 
```