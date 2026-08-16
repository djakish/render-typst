import './style.css'
import lin_font_r from '../assets/LinLibertine_R.ttf'
import lin_font_rb from '../assets/LinLibertine_RB.ttf'
import lin_font_rbi from '../assets/LinLibertine_RBI.ttf'
import test from '../assets/test.png'

import { addFont, addFile, setInputs, addSource, preparePackages, renderSvgMerged, renderPdf } from '@djakish/render-typst'

// Fonts and files are downloaded, so they have to be awaited before rendering.
const ready = Promise.all([
  addFont(lin_font_r),
  addFont(lin_font_rb),
  addFont(lin_font_rbi),
  // Adding other type of files to the world, .typ sources included
  addFile(test, "test.png"),
])

const SOURCE = `#import "@preview/cetz:0.4.2"

#text([Hello #sys.inputs.name!], fill: red)
#image("test.png", width: 40%)

#cetz.canvas({
  import cetz.draw: *

  circle((0, 0), radius: 1)
  line((-1, 0), (1, 0))

  for i in range(6) {
    let angle = i * 60deg
    line((0, 0), (calc.cos(angle) * 2.5, calc.sin(angle) * 2.5), stroke: blue)
  }

  content((0, -2), [Drawn with Cetz])
})
`

document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
  <div>
    <h1>Typst</h1>
    <div class="card">
      <button id="renderer" type="button">Render</button>
      <button id="pdf" type="button">Download PDF</button>
    </div>
    <div id="preview"></div>
  </div>
`

const preview = document.querySelector<HTMLDivElement>('#preview')!;

// Everything both buttons need before the document can be rendered.
async function compile() {
  await ready

  // Set input values, this is how add additional inputs
  setInputs({
    "name": "world",
  })

  // Set the main source file
  addSource(SOURCE, "main.typ")

  // Downloads the @preview packages the source imports, and the packages those
  // import. Only needed when packages are used, and only downloads what it has
  // not seen before.
  await preparePackages()
}

document.querySelector<HTMLButtonElement>('#renderer')!.onclick = async () => {
  try {
    await compile()
    preview.innerHTML = renderSvgMerged()
  } catch (error) {
    preview.textContent = String(error)
  }
};

document.querySelector<HTMLButtonElement>('#pdf')!.onclick = async () => {
  try {
    await compile()

    // The same document as a PDF, with the Cetz drawing in it. The cast is
    // because TypeScript types the returned buffer as ArrayBufferLike, while
    // Blob wants an ArrayBuffer.
    const pdf = renderPdf()
    const blob = new Blob([pdf.buffer as ArrayBuffer], { type: 'application/pdf' })

    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = 'drawing.pdf'
    link.click()
    URL.revokeObjectURL(url)
  } catch (error) {
    preview.textContent = String(error)
  }
};
