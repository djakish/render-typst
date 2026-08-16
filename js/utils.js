export async function fetchDataAsByteArray(dataUrl) {
    const response = await fetch(dataUrl);
    if (!response.ok) {
        throw new Error(`Failed to fetch ${dataUrl}: ${response.status} ${response.statusText}`);
    }
    const arrayBuffer = await response.arrayBuffer();
    return new Uint8Array(arrayBuffer);
}

// Downloads a package archive and, when it is gzipped, unpacks it with the
// browser's native DecompressionStream, so no decompressor has to be shipped in
// the wasm. Returns the plain tar, which is unpacked on the Rust side.
export async function fetchPackage(url) {
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`Failed to download package from ${url}: ${response.status} ${response.statusText}`);
    }
    const bytes = new Uint8Array(await response.arrayBuffer());

    // Gzip magic number. A plain .tar is passed through as it is.
    if (bytes[0] !== 0x1f || bytes[1] !== 0x8b) {
        return bytes;
    }

    const tar = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("gzip"));
    return new Uint8Array(await new Response(tar).arrayBuffer());
}
