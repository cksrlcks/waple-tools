import type { OutputPolicy } from "@/entities/tool"
import type { ThemeSetting } from "@/shared/lib/resolve-theme"

export type Settings = {
  theme: ThemeSetting
  /** 0이면 CPU 코어 수 - 1 */
  threads: number
  defaultPolicy: OutputPolicy
  keepIfLarger: boolean
  /** 이미지 도구의 메타데이터 기본값 (false = 지우기) */
  keepMetadata: boolean
  /** 0이면 정리하지 않음 */
  logRetentionDays: number
  favorites: string[]
  /** 도구 id → 마지막으로 쓴 옵션 */
  lastOptions: Record<string, unknown>
}
