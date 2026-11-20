import { useQuery } from "@tanstack/react-query"

import { iconCollections, iconSetKeys } from "@/entities/icon-set"

/** 대화상자를 열 때만 네트워크를 쓴다 */
export function useCollections(open: boolean) {
  return useQuery({
    queryKey: iconSetKeys.collections(),
    queryFn: iconCollections,
    enabled: open,
    staleTime: Infinity,
    retry: false,
  })
}
