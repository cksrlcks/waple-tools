import { useQuery } from "@tanstack/react-query"

import { readIconSet } from "./icon-set-api"
import { iconSetKeys } from "./icon-set-keys"

export function useIconSet(prefix: string | null) {
  return useQuery({
    queryKey: iconSetKeys.set(prefix ?? ""),
    queryFn: () => readIconSet(prefix!),
    enabled: !!prefix,
    staleTime: Infinity,
  })
}
