build:
    wasm-pack build --scope djakish
    jq '.files = ["*"]' pkg/package.json > tmp.json && mv tmp.json pkg/package.json
    jq '.main = "render_typst.js"' pkg/package.json > tmp.json && mv tmp.json pkg/package.json
    sed -i '1d' pkg/render_typst.js
    sed  -i '1i import * as wasm from "./render_typst_bg.wasm?init";' pkg/render_typst.js

publish:
    cd pkg && npm publish --access=public
