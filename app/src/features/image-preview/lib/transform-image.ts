/**
 * 화면용 이미지에 회전(시계 방향)·뒤집기를 적용한 새 이미지 (data URL).
 * 도구와 같은 순서: 회전 → 뒤집기. 아무것도 없으면 그대로.
 */
export async function transformImage(
  url: string,
  angle: number,
  flipH: boolean,
  flipV: boolean,
): Promise<string> {
  if (angle === 0 && !flipH && !flipV) return url
  const img = new Image()
  img.src = url
  await img.decode()
  const turned = angle % 180 !== 0
  const w = turned ? img.naturalHeight : img.naturalWidth
  const h = turned ? img.naturalWidth : img.naturalHeight
  const canvas = document.createElement("canvas")
  canvas.width = w
  canvas.height = h
  const ctx = canvas.getContext("2d")
  if (!ctx) return url
  // 캔버스 변환은 뒤에 쓴 것부터 점에 적용된다: 가운데로 옮김 → 회전 → 뒤집기
  ctx.translate(w / 2, h / 2)
  ctx.scale(flipH ? -1 : 1, flipV ? -1 : 1)
  ctx.rotate((angle * Math.PI) / 180)
  ctx.drawImage(img, -img.naturalWidth / 2, -img.naturalHeight / 2)
  return canvas.toDataURL("image/jpeg", 0.9)
}
