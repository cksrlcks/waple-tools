import { useQuery } from "@tanstack/react-query"

import { thumbnail } from "./preview-api"

export function useThumbnail(path: string, enabled: boolean) {
  return useQuery({
    queryKey: ["thumbnail", path],
    queryFn: () => thumbnail(path),
    enabled,
    staleTime: Infinity,
    retry: false,
  })
}
