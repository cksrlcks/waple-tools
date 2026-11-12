import { useMemo } from "react"
import { keepPreviousData, useQuery } from "@tanstack/react-query"

import { useDebouncedValue } from "@/shared/lib/use-debounced-value"

import { runInteractive } from "./interactive-api"
import { interactiveKeys } from "./interactive-keys"

/**
 * 파일이나 옵션이 바뀌면 300ms 뒤 다시 실행한다. 그동안 이전 결과를 보여 준다.
 * `noInput`(`x-input: "none"`)이면 파일 없이 바로 실행한다.
 * `opts`는 바뀔 때만 새 객체로 넘긴다(useMemo): 여러 줄 글은 커서 멈춘 뒤에만 문자열로 바꾼다.
 */
export function useInteractiveResult(
  toolId: string,
  path: string | null,
  opts: Record<string, unknown>,
  noInput = false,
) {
  const debounced = useDebouncedValue(opts, 300)
  const key = useMemo(() => JSON.stringify(debounced), [debounced])
  return useQuery({
    queryKey: interactiveKeys.run(toolId, path, key),
    queryFn: () => runInteractive(toolId, path, debounced),
    enabled: noInput || path !== null,
    placeholderData: keepPreviousData,
    retry: false,
    staleTime: Infinity,
  })
}
