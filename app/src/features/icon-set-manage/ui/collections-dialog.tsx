import { useState } from "react"

import type { InstalledSet } from "@/entities/icon-set"
import { toUserMessage } from "@/shared/lib/to-user-message"
import { Button } from "@/shared/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog"
import { ErrorBanner } from "@/shared/ui/error-banner"
import { Input } from "@/shared/ui/input"

import { useCollections } from "../api/use-collections"
import { filterCollections } from "../lib/filter-collections"
import { CollectionRow } from "./collection-row"

type Props = {
  open: boolean
  onOpenChange: (open: boolean) => void
  installed: InstalledSet[]
}

/** 무료 아이콘 세트 목록 — 열 때만 인터넷에서 가져온다 */
export function CollectionsDialog({ open, onOpenChange, installed }: Props) {
  const [query, setQuery] = useState("")
  const { data, error, isPending } = useCollections(open)
  const have = new Set(installed.map((s) => s.prefix))
  const rows = filterCollections(data ?? [], query)
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>아이콘 세트 더 보기</DialogTitle>
          <DialogDescription>
            무료로 쓸 수 있는 세트만 보여 줍니다.
          </DialogDescription>
        </DialogHeader>
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="세트 이름 검색"
          aria-label="세트 이름 검색"
        />
        {error && <ErrorBanner message={toUserMessage(error)} />}
        <ul className="border-border max-h-80 divide-y overflow-y-auto rounded-md border">
          {isPending && !error && (
            <li className="text-muted-foreground p-3 text-sm">불러오는 중…</li>
          )}
          {rows.map((c) => (
            <CollectionRow
              key={c.prefix}
              collection={c}
              installed={have.has(c.prefix)}
            />
          ))}
        </ul>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            닫기
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
