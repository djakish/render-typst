build:
    wasm-pack build --scope djakish --target bundler 
    jq '.files = ["*"]' pkg/package.json > tmp.json && mv tmp.json pkg/package.json

publish:
    cd pkg && npm publish --access=public

demo:   
    cd demo && pnpm install && pnpm run dev

