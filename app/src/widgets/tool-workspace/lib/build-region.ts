import type { RegionEdit } from "@/features/image-preview"
import {
  angleValue,
  ratioValue,
  regionActive,
  type RegionKeys,
} from "@/features/tool-options"

/** 스키마 `x-region`과 지금 옵션 값으로 미리보기 영역 편집 정보를 만든다. 꺼져 있으면 null */
export function buildRegion(
  keys: RegionKeys | null,
  values: Record<string, unknown>,
  set: (patch: Record<string, unknown>) => void,
): RegionEdit | null {
  if (!keys || !regionActive(keys, values)) return null
  return {
    ratio: keys.ratio ? ratioValue(values[keys.ratio]) : null,
    angle: keys.angle ? angleValue(values[keys.angle]) : 0,
    flipH: keys.flipH ? values[keys.flipH] === true : false,
    flipV: keys.flipV ? values[keys.flipV] === true : false,
    onChange: (r) =>
      set({ [keys.x]: r.x, [keys.y]: r.y, [keys.w]: r.w, [keys.h]: r.h }),
  }
}
