import {
  THEME_OPTIONS,
  useSaveSettings,
  useSettings,
  type Settings,
} from "@/entities/settings"
import { useTools } from "@/entities/tool"
import {
  AddonActions,
  addonSummary,
  isInstalledAddon,
} from "@/features/addon-install"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { cn } from "@/shared/lib/utils"
import { ErrorBanner } from "@/shared/ui/error-banner"
import { Switch } from "@/shared/ui/switch"

import {
  METADATA_OPTIONS,
  POLICY_OPTIONS,
  RETENTION_OPTIONS,
} from "../config/options"
import { ChoiceSelect } from "./choice-select"
import { SettingGroup } from "./setting-group"
import { SettingRow } from "./setting-row"

type Props = {
  className?: string
}

export function SettingsPanel({ className }: Props) {
  const settings = useSettings()
  const save = useSaveSettings()
  const addons = (useTools().data ?? []).filter((t) =>
    isInstalledAddon(t.status),
  )

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
    <div className={cn("flex flex-col gap-5", className)}>
      {save.isError && (
        <ErrorBanner
          message="설정을 저장할 수 없습니다."
          className="rounded-md border"
        />
      )}
      <SettingGroup title="일반">
        <SettingRow label="테마">
          <ChoiceSelect
            options={THEME_OPTIONS}
            value={s.theme}
            onChange={(v) => update({ theme: v as Settings["theme"] })}
          />
        </SettingRow>
        <SettingRow
          label="창을 닫으면 트레이로 보관"
          description="닫기를 눌러도 끝내지 않고 알림 영역에 남깁니다. 끝내려면 파일 › 종료."
        >
          <Switch
            aria-label="창을 닫으면 트레이로 보관"
            checked={s.closeToTray}
            onCheckedChange={(c) => update({ closeToTray: c === true })}
          />
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="작업 기본값">
        <SettingRow label="저장 방식">
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
        <SettingRow label="메타데이터">
          <ChoiceSelect
            options={METADATA_OPTIONS}
            width="short"
            value={s.keepMetadata ? "keep" : "strip"}
            onChange={(v) => update({ keepMetadata: v === "keep" })}
          />
        </SettingRow>
        <SettingRow
          label="결과가 원본보다 크면 원본 유지"
          description="용량이 오히려 늘어난 파일은 바꾸지 않습니다."
        >
          <Switch
            aria-label="결과가 원본보다 크면 원본 유지"
            checked={s.keepIfLarger}
            onCheckedChange={(c) => update({ keepIfLarger: c === true })}
          />
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="추가 도구">
        {addons.length === 0 ? (
          <p className="text-muted-foreground py-2 text-xs">
            설치한 추가 도구가 없습니다. 사이드바에서 "설치 필요" 도구를 눌러
            설치합니다.
          </p>
        ) : (
          addons.map((t) => (
            <SettingRow key={t.id} label={t.name} description={addonSummary(t)}>
              <AddonActions tool={t} />
            </SettingRow>
          ))
        )}
      </SettingGroup>

      <SettingGroup title="작업 기록">
        <SettingRow
          label="보관 기간"
          description="되돌리기에 쓰는 기록입니다. 지나면 앱을 켤 때 지웁니다."
        >
          <ChoiceSelect
            options={RETENTION_OPTIONS}
            width="short"
            value={String(s.logRetentionDays)}
            onChange={(v) => update({ logRetentionDays: Number(v) })}
          />
        </SettingRow>
      </SettingGroup>
    </div>
  )
}
