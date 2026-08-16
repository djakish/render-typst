build:
    wasm-pack build --scope djakish --target bundler
    jq '.files = ["*"]' pkg/package.json > tmp.json && mv tmp.json pkg/package.json

publish:
    cd pkg && npm publish --access=public

demo:
    cd demo && pnpm install && pnpm run dev

check:
    cargo clippy --target wasm32-unknown-unknown

# What the browser actually downloads, once the server has compressed it.
size:
    @ls -l pkg/*.wasm | awk '{printf "raw    %6.1f MB\n", $5/1048576}'
    @gzip -9 -c pkg/*.wasm | wc -c | awk '{printf "gzip   %6.1f MB\n", $1/1048576}'
    @brotli -c pkg/*.wasm | wc -c | awk '{printf "brotli %6.1f MB\n", $1/1048576}'
