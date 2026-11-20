import type { Collection } from "@/entities/icon-set"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { useInstallSet } from "../api/use-install-set"

type Props = {
  collection: Collection
  installed: boolean
  className?: string
}

export function CollectionRow({ collection, installed, className }: Props) {
  const install = useInstallSet()
  return (
    <li className={cn("flex items-center gap-3 px-3 py-2", className)}>
      <div className="min-w-0 flex-1">
        <p className="text-foreground truncate text-sm">{collection.name}</p>
        <p className="text-muted-foreground truncate text-xs">
          {[collection.category, `${collection.total}개`, collection.license]
            .filter(Boolean)
            .join(" · ")}
        </p>
      </div>
      {installed ? (
        <span className="text-muted-foreground text-xs">받음</span>
      ) : (
        <Button
          size="sm"
          variant="outline"
          disabled={install.isPending}
          onClick={() => install.mutate(collection.prefix)}
        >
          {install.isPending ? "받는 중…" : "받기"}
        </Button>
      )}
    </li>
  )
}
