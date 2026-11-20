import { useEffect, useRef } from "react"

import type { IconItem, IconSet } from "@/entities/icon-set"
import { cn } from "@/shared/lib/utils"

import { usePagedCount } from "../model/use-paged-count"
import { IconCell } from "./icon-cell"

type Props = {
  icons: IconItem[]
  set: IconSet
  selectedName: string | null
  onSelect: (icon: IconItem) => void
  className?: string
}

/** 아이콘 격자: PAGE_SIZE개씩 그리고, 맨 아래가 보이면 다음 묶음 */
export function IconGrid({
  icons,
  set,
  selectedName,
  onSelect,
  className,
}: Props) {
  const { count, more } = usePagedCount(icons.length)
  const sentinel = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const el = sentinel.current
    if (!el || count >= icons.length) return
    // count가 바뀔 때마다 다시 관찰: 더 그린 뒤에도 맨 아래가 보이면 바로 다음 묶음
    const io = new IntersectionObserver(([entry]) => {
      if (entry?.isIntersecting) more()
    })
    io.observe(el)
    return () => io.disconnect()
  }, [count, icons.length, more])

  if (icons.length === 0)
    return (
      <p className={cn("text-stage-muted m-auto text-sm", className)}>
        찾는 아이콘이 없습니다.
      </p>
    )

  return (
    <div className={cn("min-h-0 flex-1 overflow-y-auto p-3", className)}>
      <div className="grid grid-cols-[repeat(auto-fill,minmax(3rem,1fr))] gap-1">
        {icons.slice(0, count).map((icon) => (
          <IconCell
            key={icon.name}
            icon={icon}
            set={set}
            selected={icon.name === selectedName}
            onSelect={onSelect}
          />
        ))}
      </div>
      <div ref={sentinel} className="h-px" />
    </div>
  )
}
