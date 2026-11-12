import type { Block } from "@/entities/tool"

export type MediaProbe = {
  /** 초 */
  duration: number
  fps: number
}

/** 도구가 돌려준 숫자 블록 `duration`·`fps` → 값. 하나라도 없거나 0 이하면 null */
export function readProbe(blocks: Block[]): MediaProbe | null {
  const num = (label: string) => {
    const b = blocks.find((b) => b.type === "number" && b.label === label)
    return b?.type === "number" && b.value > 0 ? b.value : null
  }
  const duration = num("duration")
  const fps = num("fps")
  return duration !== null && fps !== null ? { duration, fps } : null
}
