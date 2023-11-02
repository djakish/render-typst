import './style.css'
import lin_font_r from '../assets/fonts/LinLibertine_R.ttf'
import lin_font_rb from '../assets/fonts/LinLibertine_RB.ttf'
import lin_font_rbi from '../assets/fonts/LinLibertine_RBI.ttf'

import  init, {addFont, setSource, renderSvgMerged } from '@djakish/render-typst'

addFont(lin_font_r)
addFont(lin_font_rb)
addFont(lin_font_rbi)

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
  setSource(`#text("Hello world!",fill: red)`);
  // Get rendered SVG
  let doc = renderSvgMerged()
  // Output it
  preview.innerHTML = doc
};

