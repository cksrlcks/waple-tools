import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { useInstallSet } from "../api/use-install-set"

type Props = {
  prefix: string
  name: string
  license: string
  className?: string
}

/** 아직 받지 않은 세트 안내 */
export function SetDownloadPanel({ prefix, name, license, className }: Props) {
  const install = useInstallSet()
  return (
    <div
      className={cn(
        "flex flex-col items-center gap-2 p-6 text-center",
        className,
      )}
    >
      <p className="text-foreground text-sm font-medium">{name}</p>
      <p className="text-muted-foreground text-xs">라이선스 {license}</p>
      <p className="text-muted-foreground text-sm">
        받으면 인터넷 없이 쓸 수 있습니다.
      </p>
      <Button
        size="lg"
        disabled={install.isPending}
        onClick={() => install.mutate(prefix)}
      >
        {install.isPending ? "받는 중…" : "받기"}
      </Button>
    </div>
  )
}
