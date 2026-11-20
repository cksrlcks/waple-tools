import type { DownloadProgress } from "@/entities/tool"

/** 0~100 정수. 전체 크기를 모르거나 기본 리소스 단계면 null (막대가 계속 움직이는 모양, 되감기지 않게) */
export function downloadPercent(p: DownloadProgress | null): number | null {
  if (!p?.total || p.step) return null
  return Math.min(100, Math.round((p.received / p.total) * 100))
}
