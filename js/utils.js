export async function fetchDataAsByteArray(fontUrl) {
    try {
      const response = await fetch(fontUrl);
      if (!response.ok) {
        throw new Error(`Failed to fetch data: ${response.statusText}`);
      }
      const blob = await response.blob();
      const arrayBuffer = await blob.arrayBuffer();
      return new Uint8Array(arrayBuffer);
    } catch (error) {
      console.error(error);
      throw error;
    }
}
  