import type { ToolEntry, ToolStatus } from "@/entities/tool"

/** 설정의 "추가 도구"에 보일 상태: 설치 기록이 있는 추가 도구 */
export function isInstalledAddon(status: ToolStatus): boolean {
  return (
    status === "installed" ||
    status === "updateAvailable" ||
    status === "updateRequired"
  )
}

/** "버전 1.0.0 · 새 버전 1.1.0" */
export function addonSummary(tool: ToolEntry): string {
  if (tool.status === "updateRequired") return "다시 설치해야 쓸 수 있습니다."
  const version = tool.version ? `버전 ${tool.version}` : ""
  if (tool.status === "updateAvailable" && tool.latest)
    return `${version} · 새 버전 ${tool.latest}`
  return version
}
