import { IconDots } from "@tabler/icons-react"

import type { InstalledSet } from "@/entities/icon-set"
import { Button } from "@/shared/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu"

import { useInstallSet } from "../api/use-install-set"
import { useRemoveSet } from "../api/use-remove-set"

type Props = {
  set: InstalledSet
  className?: string
}

export function SetActionsMenu({ set, className }: Props) {
  const install = useInstallSet()
  const remove = useRemoveSet()
  const hasUpdate = !!set.latest && set.latest !== set.version
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="ghost"
            size="icon"
            aria-label={`${set.name} 메뉴`}
            className={className}
          />
        }
      >
        <IconDots />
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-max min-w-28">
        {hasUpdate && (
          <DropdownMenuItem onClick={() => install.mutate(set.prefix)}>
            업데이트
          </DropdownMenuItem>
        )}
        <DropdownMenuItem onClick={() => remove.mutate(set.prefix)}>
          삭제
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
