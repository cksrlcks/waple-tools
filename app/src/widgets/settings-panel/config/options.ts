export type Option = { value: string; label: string }

export const THEME_OPTIONS: Option[] = [
  { value: "system", label: "시스템 설정 따르기" },
  { value: "light", label: "라이트" },
  { value: "dark", label: "다크" },
]

/** "다른 폴더에 저장"은 작업마다 폴더를 고르므로 기본값 후보에서 뺀다 */
export const POLICY_OPTIONS: Option[] = [
  {
    value: "InPlaceWithBackup",
    label: "원본 자리에 저장 (원본은 original 폴더로)",
  },
  { value: "KeepOriginal", label: "원본 옆에 저장" },
]

export const RETENTION_OPTIONS: Option[] = [
  { value: "7", label: "7일" },
  { value: "30", label: "30일" },
  { value: "90", label: "90일" },
  { value: "0", label: "지우지 않음" },
]

export const METADATA_OPTIONS: Option[] = [
  { value: "strip", label: "지우기" },
  { value: "keep", label: "유지" },
]
