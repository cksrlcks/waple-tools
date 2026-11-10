import { useQuery } from "@tanstack/react-query"

import { thumbnail } from "./preview-api"
import { previewKeys } from "./preview-keys"

export function useThumbnail(path: string, enabled: boolean) {
  return useQuery({
    queryKey: previewKeys.thumbnail(path),
    queryFn: () => thumbnail(path),
    enabled,
    staleTime: Infinity,
    retry: false,
  })
}
