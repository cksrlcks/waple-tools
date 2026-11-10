import type { JobSpec } from "@/entities/tool"

/** Phase 1 기본값: 원본 백업 방식, 하위 폴더 제외 (정책 선택 UI는 Phase 2) */
export function defaultSpec(inputs: string[]): JobSpec {
  return {
    inputs,
    recursive: false,
    policy: { type: "InPlaceWithBackup" },
    keepIfLarger: true,
    threads: 0,
    opts: {},
  }
}
