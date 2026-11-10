import type { ToolEntry } from "@/entities/tool"

const norm = (s: string) => s.replace(/\s+/g, "").toLowerCase()

/** 이름·설명·id에 검색어가 들어 있는 도구만 (공백·대소문자 무시) */
export function filterTools(tools: ToolEntry[], query: string): ToolEntry[] {
  const q = norm(query)
  if (!q) return tools
  return tools.filter((t) =>
    [t.name, t.description, t.id].some((s) => norm(s).includes(q)),
  )
}
