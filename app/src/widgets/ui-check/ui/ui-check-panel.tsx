// Phase 2에서 삭제: UI 이식 확인용
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/shared/ui/context-menu"
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/shared/ui/dialog"

type Props = {
  className?: string
}

const VARIANTS = [
  "default",
  "outline",
  "destructive",
  "secondary",
  "ghost",
  "link",
] as const

export function UiCheckPanel({ className }: Props) {
  return (
    <section className={cn("flex flex-col gap-3 border-t pt-3", className)}>
      <h2 className="text-2xs text-muted-foreground font-medium">UI 확인</h2>

      <div className="flex flex-wrap items-center gap-2">
        <Button
          variant="outline"
          onClick={() => document.documentElement.classList.toggle("dark")}
        >
          다크 모드 전환
        </Button>
        {VARIANTS.map((v) => (
          <Button key={v} variant={v}>
            {v}
          </Button>
        ))}
      </div>

      <Dialog>
        <DialogTrigger
          render={<Button variant="outline" className="self-start" />}
        >
          다이얼로그 열기
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>확인</DialogTitle>
            <DialogDescription>다이얼로그 표시 확인</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose render={<Button variant="outline" />}>
              취소
            </DialogClose>
            <DialogClose render={<Button />}>확인</DialogClose>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ContextMenu>
        <ContextMenuTrigger className="text-muted-foreground flex h-24 items-center justify-center rounded-md border border-dashed">
          여기를 우클릭
        </ContextMenuTrigger>
        <ContextMenuContent>
          <ContextMenuItem>열기</ContextMenuItem>
          <ContextMenuItem>이름 바꾸기</ContextMenuItem>
          <ContextMenuSeparator />
          <ContextMenuItem variant="destructive">삭제</ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>
    </section>
  )
}
