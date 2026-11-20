/** SVG 코드를 data URI 이미지로 읽어 canvas에 그린 뒤 PNG로 (투명 배경) */
export function svgToPngBlob(
  svg: string,
  px: { width: number; height: number },
): Promise<Blob> {
  return new Promise((resolve, reject) => {
    const img = new Image()
    img.onload = () => {
      const canvas = document.createElement("canvas")
      canvas.width = px.width
      canvas.height = px.height
      const ctx = canvas.getContext("2d")
      if (!ctx) return reject(new Error("canvas를 쓸 수 없습니다."))
      ctx.drawImage(img, 0, 0, px.width, px.height)
      canvas.toBlob(
        (blob) =>
          blob ? resolve(blob) : reject(new Error("PNG를 만들 수 없습니다.")),
        "image/png",
      )
    }
    img.onerror = () => reject(new Error("SVG를 읽을 수 없습니다."))
    img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`
  })
}
