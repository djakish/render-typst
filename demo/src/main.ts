import './style.css'
import lin_font_r from '../assets/LinLibertine_R.ttf'
import lin_font_rb from '../assets/LinLibertine_RB.ttf'
import lin_font_rbi from '../assets/LinLibertine_RBI.ttf'
import test from '../assets/test.png'

import { addFont, addFile, setInputs, addSource, renderSvgMerged } from '@djakish/render-typst'

addFont(lin_font_r)
addFont(lin_font_rb)
addFont(lin_font_rbi)

// Adding other type of files to the world
// .typ files don't work
addFile(test, "test.png")

document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
  <div>
    <h1>Typst</h1>
    <div class="card">
      <button id="renderer" type="button">Render</button>
    </div>
    <div id="preview"></div>
  </div>
`

const button = document.querySelector<HTMLButtonElement>('#renderer')!;

button.onclick = () => {
  let preview = document.querySelector<HTMLDivElement>('#preview')!;

  // Set input values, this is how add additional inputs
  setInputs({
    "name": "world",
  })

  // Set the main source file
  addSource(
    `#text([Hello #sys.inputs.name!],fill: red)
     #image("test.png")
    `, 
    "main.typ"
  )

  // Get rendered SVG
  let doc = renderSvgMerged()

  // Output it
  preview.innerHTML = doc
};

