import { Button } from "@/shared/ui/button"
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog"

import { SNIPPET_GROUPS } from "../config/regex-snippets"
import { SnippetButton } from "./snippet-button"

type Props = {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** 누른 토큰을 패턴에 넣는다 (넣으면 창을 닫는다) */
  onInsert: (insert: string) => void
}

/** 정규식 치트 시트 전체: 분류별 토큰 */
export function RegexCheatsheetDialog({ open, onOpenChange, onInsert }: Props) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>치트 시트</DialogTitle>
        </DialogHeader>
        <div className="flex flex-col gap-4 pb-2">
          {SNIPPET_GROUPS.map((g) => (
            <section key={g.title} className="flex flex-col gap-1.5">
              <h3 className="text-muted-foreground text-xs">{g.title}</h3>
              <div className="flex flex-wrap items-center gap-1.5">
                {g.items.map((s) => (
                  <SnippetButton
                    key={s.label}
                    snippet={s}
                    onInsert={(v) => {
                      onInsert(v)
                      onOpenChange(false)
                    }}
                  />
                ))}
              </div>
            </section>
          ))}
        </div>
        <DialogFooter>
          <Button
            variant="outline"
            className="px-5"
            onClick={() => onOpenChange(false)}
          >
            닫기
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
