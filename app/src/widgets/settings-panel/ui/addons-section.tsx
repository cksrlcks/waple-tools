import { useTools } from "@/entities/tool"
import {
  AddonActions,
  addonSummary,
  isInstalledAddon,
} from "@/features/addon-install"

import { SettingGroup } from "./setting-group"
import { SettingRow } from "./setting-row"

export function AddonsSection() {
  const addons = (useTools().data ?? []).filter((t) =>
    isInstalledAddon(t.status),
  )
  return (
    <SettingGroup title="설치한 도구">
      {addons.length === 0 ? (
        <p className="text-muted-foreground py-3 text-xs">
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
  )
}
