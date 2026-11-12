import { useState } from "react"

import { Dialog, DialogContent, DialogTitle } from "@/shared/ui/dialog"

import { SETTINGS_SECTIONS, type SettingsSectionId } from "../config/sections"
import { SettingsNav } from "./settings-nav"
import { SettingsPanel } from "./settings-panel"

type Props = {
  open: boolean
  onClose: () => void
}

/** 파일 › 설정… 에서 여는 설정 창: 왼쪽 섹션 목록, 오른쪽 내용 (바꾸면 바로 저장) */
export function SettingsDialog({ open, onClose }: Props) {
  const [section, setSection] = useState<SettingsSectionId>("general")
  const label = SETTINGS_SECTIONS.find((s) => s.id === section)?.label

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      {/* footer가 없는 창이라 ✕로 닫는다 (UI-SYSTEM Dialog) */}
      <DialogContent
        showCloseButton
        className="flex h-130 gap-0 overflow-hidden p-0 sm:max-w-3xl"
      >
        <aside className="border-border flex w-52 shrink-0 flex-col gap-4 border-r p-3 pt-4.5">
          <DialogTitle className="px-2.5">설정</DialogTitle>
          <SettingsNav value={section} onChange={setSection} />
        </aside>
        <div className="flex min-w-0 flex-1 flex-col">
          <header className="border-border text-muted-foreground flex h-14 shrink-0 items-center gap-2 border-b px-6 text-sm">
            설정
            <span aria-hidden>/</span>
            <span className="text-foreground">{label}</span>
          </header>
          <SettingsPanel
            section={section}
            className="flex-1 overflow-y-auto px-6 py-5"
          />
        </div>
      </DialogContent>
    </Dialog>
  )
}
