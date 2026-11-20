import { useQuery } from "@tanstack/react-query"

import { toolResources } from "./tool-resource-api"
import { toolResourceKeys } from "./tool-resource-keys"

/** 받은 리소스 목록 (로컬 호출, 네트워크 없음). 설치된 도구만 `enabled` */
export function useToolResources(toolId: string, enabled: boolean) {
  return useQuery({
    queryKey: toolResourceKeys.list(toolId),
    queryFn: () => toolResources(toolId),
    enabled,
  })
}
