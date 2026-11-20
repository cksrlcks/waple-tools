import type { DownloadProgress } from "@/entities/tool"

import { downloadPercent } from "./download-percent"

/** 설치 진행 문구. 도구 파일은 받은 비율, 다음 단계는 "n/전체 (이름)" */
export function progressText(p: DownloadProgress | null): string {
  if (p?.step)
    return `기본 리소스 받는 중 ${p.step.index}/${p.step.count} (${p.step.label})`
  const pct = downloadPercent(p)
  return pct === null ? "받는 중…" : `받는 중… ${pct}%`
}
