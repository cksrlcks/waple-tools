import { useMemo } from "react"
import { keepPreviousData, useQuery } from "@tanstack/react-query"

import { interactiveKeys, type Block } from "@/entities/tool"
import { useDebouncedValue } from "@/shared/lib/use-debounced-value"

import { mergePanelPart } from "../lib/merge-panel-part"
import { runInteractive } from "./interactive-api"

/** `x-panel-keys`(`keys`)와 패널 블록 종류(`x-panel-blocks`) */
type PanelPart = { keys: string[]; types: Block["type"][] }

const NO_PART: PanelPart = { keys: [], types: [] }

/**
 * 파일이나 옵션이 바뀌면 300ms 뒤 다시 실행한다. 그동안 이전 결과를 보여 준다.
 * `noInput`(`x-input: "none"`)이면 파일 없이 바로 실행한다.
 * `opts`는 바뀔 때만 새 객체로 넘긴다(useMemo): 여러 줄 글은 커서 멈춘 뒤에만 문자열로 바꾼다.
 *
 * `panel.keys`가 있으면 둘로 나눈다: 전체 실행은 그 키를 뺀 옵션으로(그 키만 바뀌면 다시 실행하지 않음),
 * 패널 실행은 모든 옵션 + `__part: "panel"`로 50ms 뒤. 결과는 `mergePanelPart`로 합친다.
 * `fetching`은 전체 실행만(작업대를 흐리게), `panelFetching`은 둘 중 하나(패널을 흐리게).
 * `panel`은 바뀔 때만 새 객체로 넘긴다.
 */
export function useInteractiveResult(
  toolId: string,
  path: string | null,
  opts: Record<string, unknown>,
  noInput = false,
  panel: PanelPart = NO_PART,
) {
  const split = panel.keys.length > 0
  const stageOpts = useMemo(
    () =>
      split
        ? Object.fromEntries(
            Object.entries(opts).filter(([k]) => !panel.keys.includes(k)),
          )
        : opts,
    [opts, split, panel.keys],
  )
  const enabled = noInput || path !== null
  const debounced = useDebouncedValue(stageOpts, 300)
  const key = useMemo(() => JSON.stringify(debounced), [debounced])
  const stage = useQuery({
    queryKey: interactiveKeys.run(toolId, path, key),
    queryFn: () => runInteractive(toolId, path, debounced),
    enabled,
    placeholderData: keepPreviousData,
    retry: false,
    staleTime: Infinity,
  })

  const panelDebounced = useDebouncedValue(opts, 50)
  const panelKey = useMemo(
    () => JSON.stringify(panelDebounced),
    [panelDebounced],
  )
  const part = useQuery({
    queryKey: [...interactiveKeys.run(toolId, path, panelKey), "panel"],
    queryFn: () =>
      runInteractive(toolId, path, { ...panelDebounced, __part: "panel" }),
    enabled: enabled && split,
    placeholderData: keepPreviousData,
    retry: false,
    staleTime: Infinity,
  })

  const data = useMemo(
    () =>
      split ? mergePanelPart(stage.data, part.data, panel.types) : stage.data,
    [split, stage.data, part.data, panel.types],
  )
  return {
    data,
    isPlaceholderData: stage.isPlaceholderData,
    isFetching: stage.isFetching,
    panelFetching: stage.isFetching || (split && part.isFetching),
    isError: stage.isError || (split && part.isError),
    error: stage.error ?? (split ? part.error : null),
  }
}
