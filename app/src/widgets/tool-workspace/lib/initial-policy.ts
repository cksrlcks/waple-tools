import type { OutputPolicy } from "@/entities/tool"

/** 도구 id의 마지막 부분으로 만든 기본 접미사 (`image.webp` → `_webp`) */
export const suffixOf = (id: string) => `_${id.split(".").pop()}`

/** 설정의 기본 저장 방식을 이 도구용으로 채운다. 다른 폴더는 작업마다 고른다. */
export function initialPolicy(p: OutputPolicy, toolId: string): OutputPolicy {
  if (p.type === "KeepOriginal" && !p.suffix)
    return { type: "KeepOriginal", suffix: suffixOf(toolId) }
  if (p.type === "OutputFolder") return { type: "OutputFolder", dir: "" }
  return p
}
