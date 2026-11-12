import { useMemo } from "react"
import { keepPreviousData, useQuery } from "@tanstack/react-query"

import type { JobSpec } from "@/entities/tool"
import { useDebouncedValue } from "@/shared/lib/use-debounced-value"

import { planRename } from "./rename-api"
import { renameKeys } from "./rename-keys"

/**
 * 입력·옵션이 바뀌면 300ms 뒤 미리보기 표를 다시 만든다. 그동안 이전 표를 보여 준다.
 * `current`: 표가 지금 입력·옵션으로 만든 것인가 (아니면 실행을 막는다)
 */
export function useRenamePlan(toolId: string, spec: JobSpec) {
  const key = JSON.stringify(spec)
  const debounced = useDebouncedValue(key, 300)
  const parsed = useMemo(() => JSON.parse(debounced) as JobSpec, [debounced])
  const query = useQuery({
    queryKey: renameKeys.plan(toolId, debounced),
    queryFn: () => planRename(toolId, parsed),
    enabled: parsed.inputs.length > 0,
    placeholderData: keepPreviousData,
    retry: false,
  })
  return {
    ...query,
    current: debounced === key && query.isSuccess && !query.isPlaceholderData,
  }
}
