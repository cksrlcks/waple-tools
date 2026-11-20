import { useState } from "react"

import type { ResourceItem } from "@/entities/tool"
import { useCheckResources, useToolResources } from "@/entities/tool-resource"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { Input } from "@/shared/ui/input"

import { ResourceRow } from "./resource-row"

type Props = {
  /** 설치된 추가 도구 id (리소스 명령은 추가 도구만) */
  toolId: string
  items: ResourceItem[]
  /** 고른 항목 id (지금 옵션 값) */
  selected?: string
  onSelect: (id: string) => void
  className?: string
}

/**
 * 리소스 목록: 위 검색(앱 안에서 제목·id로 거르기), 줄마다 받기·업데이트·삭제, 아래 [업데이트 확인].
 * 받은 목록은 로컬에서 읽고, 네트워크는 버튼을 누를 때만 쓴다.
 */
export function ResourceListBlock({
  toolId,
  items,
  selected,
  onSelect,
  className,
}: Props) {
  const [query, setQuery] = useState("")
  const resources = useToolResources(toolId, true)
  const check = useCheckResources(toolId)
  const states = new Map((resources.data ?? []).map((r) => [r.id, r]))
  const q = query.trim().toLowerCase()
  const shown = q
    ? items.filter((i) => i.title.toLowerCase().includes(q) || i.id.includes(q))
    : items

  return (
    <div className={cn("flex min-h-0 flex-col gap-2", className)}>
      <Input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="목록 검색"
        aria-label="목록 검색"
        className="shrink-0"
      />
      <ul className="-mx-1 min-h-0 flex-1 overflow-y-auto px-1">
        {shown.map((item) => (
          <ResourceRow
            key={item.id}
            toolId={toolId}
            item={item}
            state={states.get(item.id)}
            selected={item.id === selected}
            onSelect={onSelect}
          />
        ))}
        {shown.length === 0 && (
          <li className="text-muted-foreground py-4 text-center text-xs">
            찾는 항목이 없습니다.
          </li>
        )}
      </ul>
      <Button
        variant="outline"
        className="shrink-0"
        disabled={states.size === 0 || check.isPending}
        onClick={() => check.mutate()}
      >
        {check.isPending ? "확인 중…" : "업데이트 확인"}
      </Button>
    </div>
  )
}
