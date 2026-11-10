import { IconRefresh } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import pkg from "../../../../package.json"
import {
  checkForUpdate,
  installUpdate,
  useAvailableVersion,
  useUpdater,
} from "../model/updater"

type Props = {
  /** 작업 중이면 설치하지 않는다 */
  locked?: boolean
  className?: string
}

/** 버전 표시 + 업데이트 확인. 새 버전이 있으면 지금 버전에 취소선을 긋고 새 버전과 업데이트 버튼을 보인다 */
export function VersionCheck({ locked = false, className }: Props) {
  const s = useUpdater()
  const latest = useAvailableVersion()
  const checking = s.kind === "checking"
  const downloading = s.kind === "downloading"

  return (
    <div
      className={cn(
        "text-muted-foreground flex h-6 items-center gap-1.5 text-xs",
        className,
      )}
    >
      {latest ? (
        <>
          <s>v{pkg.version}</s>
          <span className="text-foreground font-medium">v{latest}</span>
          <Button
            size="xs"
            disabled={downloading || locked}
            title={
              locked ? "작업이 끝난 뒤 업데이트할 수 있습니다." : undefined
            }
            onClick={() => void installUpdate()}
          >
            {downloading ? "업데이트 중…" : "업데이트"}
          </Button>
        </>
      ) : (
        <span>v{pkg.version}</span>
      )}
      <Button
        variant="ghost"
        size="icon-xs"
        disabled={checking || downloading}
        onClick={() => void checkForUpdate(true)}
        title="업데이트 확인"
        aria-label="업데이트 확인"
      >
        <IconRefresh className={checking ? "animate-spin" : undefined} />
      </Button>
    </div>
  )
}
