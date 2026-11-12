import {
  useSaveSettings,
  useSettings,
  type Settings,
} from "@/entities/settings"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { ErrorBanner } from "@/shared/ui/error-banner"

import type { SettingsSectionId } from "../config/sections"
import { AddonsSection } from "./addons-section"
import { DefaultsSection } from "./defaults-section"
import { GeneralSection } from "./general-section"

type Props = {
  section: SettingsSectionId
  className?: string
}

/** 고른 섹션의 설정 (바꾸면 바로 저장) */
export function SettingsPanel({ section, className }: Props) {
  const settings = useSettings()
  const save = useSaveSettings()

  if (!settings.data) {
    return settings.isError ? (
      <ErrorBanner message={toUserMessage(settings.error)} />
    ) : (
      <p className="text-muted-foreground">불러오는 중…</p>
    )
  }
  const s = settings.data
  const update = (patch: Partial<Settings>) => save.mutate({ ...s, ...patch })

  return (
    <div className={cn("flex flex-col gap-6", className)}>
      {save.isError && (
        <ErrorBanner
          message="설정을 저장할 수 없습니다."
          className="rounded-md border"
        />
      )}
      {section === "general" && <GeneralSection settings={s} update={update} />}
      {section === "defaults" && (
        <DefaultsSection settings={s} update={update} />
      )}
      {section === "addons" && <AddonsSection />}
    </div>
  )
}
