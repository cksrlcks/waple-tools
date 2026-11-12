import {
  IconAdjustmentsHorizontal,
  IconPuzzle,
  IconSettings,
} from "@tabler/icons-react"

export const SETTINGS_SECTIONS = [
  { id: "general", label: "일반", icon: IconSettings },
  { id: "defaults", label: "작업 기본값", icon: IconAdjustmentsHorizontal },
  { id: "addons", label: "추가 도구", icon: IconPuzzle },
] as const

export type SettingsSectionId = (typeof SETTINGS_SECTIONS)[number]["id"]
