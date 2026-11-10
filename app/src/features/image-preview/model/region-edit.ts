import type { Rect } from "../lib/scale-crop"

/** 미리보기 위에서 영역을 끌어 정할 때 필요한 값 (스키마 `x-region`에서 만든다) */
export type RegionEdit = {
  /** 가로/세로 비율 고정, null이면 자유 */
  ratio: number | null
  /** 시계 방향 각도 (0·90·180·270) */
  angle: number
  flipH: boolean
  flipV: boolean
  /** 고른 영역 (회전·뒤집기 뒤 원본 기준 px) */
  onChange: (rect: Rect) => void
}
