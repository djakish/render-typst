
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

Rendering runs Typst 0.15. Anything that fails — a syntax error, a missing
file, a package that will not download — throws with the compiler's message,
so wrap the render calls in `try`/`catch`.

# Packages

Packages from the [`@preview` namespace](https://typst.app/universe/) work, but
they have to be downloaded first, and downloading is async while rendering is
not. So after the sources are set, call `preparePackages()` once:

```ts
import { addSource, preparePackages, renderSvgMerged } from '@djakish/render-typst'

addSource(`
  #import "@preview/cetz:0.4.2"

  #cetz.canvas({
    import cetz.draw: *
    circle((0, 0), radius: 1)
  })
`, "main.typ")

await preparePackages()

let doc = renderSvgMerged()
```

It reads the sources you added, downloads what they import from
[packages.typst.org](https://packages.typst.org) (including packages that those
packages import), and keeps them for the rest of the session. Calling it again
after an edit only downloads what is new, so it is cheap to call before every
render. Archives are gunzipped by the browser's `DecompressionStream`, which
keeps the decompressor out of the wasm.

## Your own packages

`addPackage` mounts a package from an archive you host yourself, under whatever
name you want to import it as:

```ts
// A .tar or .tar.gz laid out like a Typst package: typst.toml at the root,
// naming the entrypoint.
await addPackage("@local/notes:1.0.0", "/packages/notes-1.0.0.tar.gz")

addSource(`#import "@local/notes:1.0.0": *`, "main.typ")
```

Any namespace works, `@preview` included — a package mounted this way is used
as it is and never downloaded. `preparePackages()` only goes to the network for
`@preview` imports it has not seen.

## Linking files

Multi-file documents need no packaging at all. Anything mounted with
`addSource` or `addFile` is importable and readable by path:

```ts
addSource(`#let hi = [Hello!]`, "helpers.typ")
await addFile("/logo.png", "images/logo.png")

addSource(`
  #import "helpers.typ": hi
  #hi
  #image("images/logo.png")
`, "main.typ")
```

`addFile` downloads from a URL and works for `.typ` sources too; they are
parsed the first time something imports them.

# Vite dependencies for wasm

With vite you need [vite-plugin-wasm](https://www.npmjs.com/package/vite-plugin-wasm).

On vite 7 and older you also need
[vite-plugin-top-level-await](https://www.npmjs.com/package/vite-plugin-top-level-await).
Do not install it on vite 8 — vite 8 builds on rolldown instead of rollup, and
that plugin requires rollup, so the dev server refuses to start. Set
`build.target` and `esbuild.target` to `esnext` instead, as `demo/vite.config.ts`
does.


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

Or `just build`, `just check` (clippy), `just demo`, `just size`.

# Size

Typst is a whole typesetting engine, so the wasm is large. What matters is what
crosses the wire:

| | |
| --- | --- |
| raw | 20.2 MB |
| gzip | 7.9 MB |
| brotli | 5.9 MB |

Serving it with brotli is worth more than any compiler flag here, so make sure
compression is on for `.wasm` — and let the browser cache it, since the file
only changes when this package is republished. `just size` prints all three.
