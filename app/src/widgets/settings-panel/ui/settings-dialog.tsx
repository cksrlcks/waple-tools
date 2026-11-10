import { Button } from "@/shared/ui/button"
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog"

import { SettingsPanel } from "./settings-panel"

type Props = {
  open: boolean
  onClose: () => void
}

/** 파일 › 설정… 에서 여는 설정 창 (바꾸면 바로 저장) */
export function SettingsDialog({ open, onClose }: Props) {
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>설정</DialogTitle>
        </DialogHeader>
        <SettingsPanel />
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            닫기
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
