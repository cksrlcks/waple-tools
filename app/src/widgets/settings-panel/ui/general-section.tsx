import { THEME_OPTIONS, type Settings } from "@/entities/settings"
import { Checkbox } from "@/shared/ui/checkbox"
import { Segmented } from "@/shared/ui/segmented"

import { SettingGroup } from "./setting-group"
import { SettingRow } from "./setting-row"

type Props = {
  settings: Settings
  update: (patch: Partial<Settings>) => void
}

export function GeneralSection({ settings: s, update }: Props) {
  return (
    <>
      <SettingGroup title="모양">
        <SettingRow
          label="테마"
          description="시스템을 고르면 Windows의 라이트·다크 설정을 따릅니다."
        >
          <Segmented
            aria-label="테마"
            options={THEME_OPTIONS}
            value={s.theme}
            onChange={(v) => update({ theme: v as Settings["theme"] })}
          />
        </SettingRow>
      </SettingGroup>
      <SettingGroup title="시작 및 종료">
        <SettingRow
          label="창을 닫으면 트레이로 보관"
          description="닫기를 눌러도 끝내지 않고 알림 영역에 남깁니다. 끝내려면 파일 › 종료."
        >
          <Checkbox
            aria-label="창을 닫으면 트레이로 보관"
            checked={s.closeToTray}
            onCheckedChange={(c) => update({ closeToTray: c })}
          />
        </SettingRow>
      </SettingGroup>
    </>
  )
}
