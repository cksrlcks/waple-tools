export type Option = { value: string; label: string }

/** "다른 폴더에 저장"은 작업마다 폴더를 고르므로 기본값 후보에서 뺀다 */
export const POLICY_OPTIONS: Option[] = [
  { value: "InPlaceWithBackup", label: "원본 자리" },
  { value: "KeepOriginal", label: "원본 옆" },
]

export const METADATA_OPTIONS: Option[] = [
  { value: "strip", label: "지우기" },
  { value: "keep", label: "유지" },
]
