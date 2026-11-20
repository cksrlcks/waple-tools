import { useEffect, useState } from "react"
import { IconX } from "@tabler/icons-react"

import { Button } from "@/shared/ui/button"

import { emitPanel, listenPanel } from "../api/panel-window"
import { changedEntries, type PanelState } from "../lib/panel-state"
import { PanelContent } from "./panel-content"

/**
 * 옵션 패널 별도 창(라벨 `panel`)의 화면: 메인이 보낸 `panel:state`를 아래 시트와 같은 내용으로 그린다.
 * 머리 줄은 끌기 영역(테두리 없는 창), 오른쪽 X는 메인에 `panel:close`. 옵션을 바꾸면 메인에 `panel:set`.
 */
export function PanelWindow() {
  const [state, setState] = useState<PanelState | null>(null)
  useEffect(
    () =>
      listenPanel({ "panel:state": (p) => setState(p as PanelState) }, () =>
        emitPanel("main", "panel:ready"),
      ),
    [],
  )

  const change = (next: Record<string, unknown>) => {
    if (!state) return
    for (const [key, value] of changedEntries(state.options, next))
      emitPanel("main", "panel:set", { key, value })
    // 메인의 답을 기다리지 않고 바로 (슬라이더가 끊기지 않게)
    setState({ ...state, options: next })
  }

  return (
    // 결과 블록의 작업대 색 토큰을 테마 색으로
    <div className="stage-plain bg-background text-foreground flex h-screen flex-col">
      <header
        data-tauri-drag-region
        className="flex h-10 shrink-0 items-center gap-2 pr-2 pl-4 select-none"
      >
        <span
          data-tauri-drag-region
          className="text-muted-foreground min-w-0 flex-1 truncate text-xs font-medium"
        >
          {state?.title}
        </span>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="닫기"
          className="text-muted-foreground"
          onClick={() => emitPanel("main", "panel:close")}
        >
          <IconX stroke={1.75} />
        </Button>
      </header>
      {state && (
        <PanelContent
          fields={state.fields}
          options={state.options}
          onChange={change}
          blocks={state.blocks}
          hints={state.hints}
          fading={state.fading}
          error={state.error}
          className="pt-1"
        />
      )}
    </div>
  )
}
