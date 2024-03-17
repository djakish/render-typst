import './style.css'
import lin_font_r from '../assets/LinLibertine_R.ttf'
import lin_font_rb from '../assets/LinLibertine_RB.ttf'
import lin_font_rbi from '../assets/LinLibertine_RBI.ttf'
import test from '../assets/test.png'

import {addFont, addFile, setSource, renderSvgMerged } from '@djakish/render-typst'

addFont(lin_font_r)
addFont(lin_font_rb)
addFont(lin_font_rbi)

// you have to manually set the name of the file
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
  // Set source to the wasm 
  setSource(`
    #text("Hello world!",fill: red)
    #image("test.png")
  `);
  // Get rendered SVG
  let doc = renderSvgMerged()
  // Output it
  preview.innerHTML = doc
};

