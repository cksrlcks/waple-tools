import {
  useSaveSettings,
  useSettings,
  type Settings,
} from "@/entities/settings"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { Checkbox } from "@/shared/ui/checkbox"
import { ErrorBanner } from "@/shared/ui/error-banner"

import {
  METADATA_OPTIONS,
  POLICY_OPTIONS,
  RETENTION_OPTIONS,
  THEME_OPTIONS,
} from "../config/options"
import { ChoiceSelect } from "./choice-select"
import { SettingRow } from "./setting-row"

type Props = {
  className?: string
}

export function SettingsPanel({ className }: Props) {
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
    <section className={cn("flex max-w-2xl flex-col gap-4", className)}>
      <h1 className="text-base font-semibold">설정</h1>
      {save.isError && (
        <ErrorBanner
          message="설정을 저장할 수 없습니다."
          className="rounded-md border"
        />
      )}
      <div className="flex flex-col">
        <SettingRow label="테마">
          <ChoiceSelect
            options={THEME_OPTIONS}
            value={s.theme}
            onChange={(v) => update({ theme: v as Settings["theme"] })}
          />
        </SettingRow>
        <SettingRow label="기본 저장 방식">
          <ChoiceSelect
            options={POLICY_OPTIONS}
            value={
              s.defaultPolicy.type === "KeepOriginal"
                ? "KeepOriginal"
                : "InPlaceWithBackup"
            }
            onChange={(v) =>
              update({
                defaultPolicy:
                  v === "KeepOriginal"
                    ? { type: "KeepOriginal", suffix: "" }
                    : { type: "InPlaceWithBackup" },
              })
            }
          />
        </SettingRow>
        <SettingRow label="기본 메타데이터 처리">
          <ChoiceSelect
            options={METADATA_OPTIONS}
            value={s.keepMetadata ? "keep" : "strip"}
            onChange={(v) => update({ keepMetadata: v === "keep" })}
          />
        </SettingRow>
        <SettingRow label="결과가 원본보다 크면 원본 유지">
          <Checkbox
            checked={s.keepIfLarger}
            onCheckedChange={(c) => update({ keepIfLarger: c === true })}
          />
        </SettingRow>
        <SettingRow label="작업 기록 보관 기간">
          <ChoiceSelect
            options={RETENTION_OPTIONS}
            value={String(s.logRetentionDays)}
            onChange={(v) => update({ logRetentionDays: Number(v) })}
          />
        </SettingRow>
      </div>
    </section>
  )
}
