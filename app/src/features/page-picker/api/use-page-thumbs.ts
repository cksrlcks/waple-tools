import { useQuery } from "@tanstack/react-query"

import { pageThumbKeys } from "./page-thumb-keys"
import { pageThumbnails } from "./page-thumbs-api"

export function usePageThumbs(toolId: string, path: string | null) {
  return useQuery({
    queryKey: pageThumbKeys.of(toolId, path ?? ""),
    queryFn: () => pageThumbnails(toolId, path as string),
    enabled: path !== null,
    staleTime: Infinity,
    retry: false,
  })
}
