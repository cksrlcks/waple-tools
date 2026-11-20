/** `data:<형식>;base64,…`를 Blob으로 (형식이 없으면 image/png) */
export function dataUrlBlob(src: string): Blob {
  const type = src.slice(5, src.indexOf(";")) || "image/png"
  const bytes = Uint8Array.from(atob(src.slice(src.indexOf(",") + 1)), (c) =>
    c.charCodeAt(0),
  )
  return new Blob([bytes], { type })
}
