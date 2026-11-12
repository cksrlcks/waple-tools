import type { Settings } from "@/entities/settings"
import { Checkbox } from "@/shared/ui/checkbox"
import { Segmented } from "@/shared/ui/segmented"

import { METADATA_OPTIONS, POLICY_OPTIONS } from "../config/options"
import { SettingGroup } from "./setting-group"
import { SettingRow } from "./setting-row"

type Props = {
  settings: Settings
  update: (patch: Partial<Settings>) => void
}

export function DefaultsSection({ settings: s, update }: Props) {
  return (
    <>
      <SettingGroup title="저장">
        <SettingRow
          label="저장 방식"
          description="원본 자리에 저장하면 원본은 original 폴더로 옮깁니다."
        >
          <Segmented
            aria-label="저장 방식"
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
        <SettingRow
          label="결과가 원본보다 크면 원본 유지"
          description="용량이 오히려 늘어난 파일은 바꾸지 않습니다."
        >
          <Checkbox
            aria-label="결과가 원본보다 크면 원본 유지"
            checked={s.keepIfLarger}
            onCheckedChange={(c) => update({ keepIfLarger: c })}
          />
        </SettingRow>
      </SettingGroup>
      <SettingGroup title="이미지">
        <SettingRow
          label="메타데이터"
          description="이미지 도구 결과에 촬영 정보 같은 메타데이터를 남길지 정합니다."
        >
          <Segmented
            aria-label="메타데이터"
            options={METADATA_OPTIONS}
            value={s.keepMetadata ? "keep" : "strip"}
            onChange={(v) => update({ keepMetadata: v === "keep" })}
          />
        </SettingRow>
      </SettingGroup>
    </>
  )
}
