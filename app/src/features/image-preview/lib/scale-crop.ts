/** 크롭 영역 (px) */
export type Rect = { x: number; y: number; w: number; h: number }
export type Size = { w: number; h: number }

/**
 * 화면용 이미지(`from`, 줄어 있을 수 있음) 기준 영역을 원본 크기(`to`) 기준 정수 px로.
 * 원본 밖으로 나가지 않고 최소 1px.
 */
export function scaleCrop(
  r: { x: number; y: number; width: number; height: number },
  from: Size,
  to: Size,
): Rect {
  const sx = to.w / from.w
  const sy = to.h / from.h
  const x = Math.min(Math.max(Math.round(r.x * sx), 0), to.w - 1)
  const y = Math.min(Math.max(Math.round(r.y * sy), 0), to.h - 1)
  return {
    x,
    y,
    w: Math.min(Math.max(Math.round(r.width * sx), 1), to.w - x),
    h: Math.min(Math.max(Math.round(r.height * sy), 1), to.h - y),
  }
}
