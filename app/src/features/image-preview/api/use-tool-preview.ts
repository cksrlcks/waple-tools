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
  const now = JSON.stringify(opts)
  const key = useDebouncedValue(now, 300)
  const debounced = useMemo(
    () => JSON.parse(key) as Record<string, unknown>,
    [key],
  )
  const query = useQuery({
    queryKey: previewKeys.tool(toolId, path, key),
    queryFn: () => previewTool(toolId, path as string, debounced),
    enabled: path !== null,
    placeholderData: keepPreviousData,
    retry: false,
    staleTime: Infinity,
  })
  // 보여 주는 결과가 지금 옵션 것이 아님: 디바운스 대기 중이거나, 새로 만드는 중(이전 결과를 유지)
  const stale = now !== key || query.isFetching || query.isPlaceholderData
  return { ...query, stale }
}
