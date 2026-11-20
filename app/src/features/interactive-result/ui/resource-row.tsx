import { IconDots, IconDownload, IconLoader2 } from "@tabler/icons-react"

import type { ResourceItem } from "@/entities/tool"
import {
  useInstallResource,
  useRemoveResource,
  type ResourceState,
} from "@/entities/tool-resource"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu"

type Props = {
  toolId: string
  item: ResourceItem
  /** 받은 상태 (받기 전이면 없음) */
  state?: ResourceState
  selected: boolean
  onSelect: (id: string) => void
  className?: string
}

/** 리소스 한 줄: 받기 전이면 [받기], 받았으면 눌러서 고르고 메뉴에서 삭제. 새 버전이 있으면 [업데이트] */
export function ResourceRow({
  toolId,
  item,
  state,
  selected,
  onSelect,
  className,
}: Props) {
  const install = useInstallResource(toolId)
  const remove = useRemoveResource(toolId)
  const installed = state !== undefined
  // 처음 받으면 바로 그 리소스를 고른다 (업데이트는 고른 것을 바꾸지 않는다)
  const get = () =>
    install.mutate(item.id, { onSuccess: () => onSelect(item.id) })
  const update = () => install.mutate(item.id)

  return (
    <li
      className={cn(
        "flex items-center gap-1 rounded-md pr-1",
        installed && "hover:bg-accent",
        selected && "bg-selection hover:bg-selection",
        className,
      )}
    >
      <button
        type="button"
        disabled={!installed}
        aria-current={selected ? "true" : undefined}
        onClick={() => onSelect(item.id)}
        className="focus-visible:ring-primary/40 flex min-w-0 flex-1 flex-col items-start rounded-md px-2 py-1.5 text-left outline-none focus-visible:ring-2 disabled:cursor-default"
      >
        <span className="w-full truncate text-sm">{item.title}</span>
        <span className="text-muted-foreground w-full truncate text-xs">
          {item.meta}
        </span>
      </button>
      {install.isPending ? (
        <span className="text-muted-foreground flex items-center gap-1 px-1 text-xs whitespace-nowrap">
          <IconLoader2 className="size-3.5 animate-spin" />
          받는 중
        </span>
      ) : !installed ? (
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={`${item.title} 받기`}
          title="받기"
          onClick={get}
        >
          <IconDownload />
        </Button>
      ) : (
        <>
          {state.latest && (
            <Button variant="outline" size="xs" onClick={update}>
              업데이트
            </Button>
          )}
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label={`${item.title} 메뉴`}
                />
              }
            >
              <IconDots />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-max min-w-28">
              <DropdownMenuItem
                disabled={remove.isPending}
                onClick={() => remove.mutate(item.id)}
              >
                삭제
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </>
      )}
    </li>
  )
}
