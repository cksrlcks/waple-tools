import { useMemo } from "react"
import { keepPreviousData, useQuery } from "@tanstack/react-query"

import { useDebouncedValue } from "@/shared/lib/use-debounced-value"

import { previewTool } from "./preview-api"
import { previewKeys } from "./preview-keys"

/** 옵션을 바꾸는 동안에는 이전 결과를 유지하고, 300ms 멈추면 다시 만든다 */
export function useToolPreview(
  toolId: string,
  path: string | null,
  opts: Record<string, unknown>,
) {
  // 옵션 객체는 렌더마다 새로 만들어지므로 내용(JSON)으로 비교한다
  const key = useDebouncedValue(JSON.stringify(opts), 300)
  const debounced = useMemo(
    () => JSON.parse(key) as Record<string, unknown>,
    [key],
  )
  return useQuery({
    queryKey: previewKeys.tool(toolId, path, key),
    queryFn: () => previewTool(toolId, path as string, debounced),
    enabled: path !== null,
    placeholderData: keepPreviousData,
    retry: false,
    staleTime: Infinity,
  })
}
