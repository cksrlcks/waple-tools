import { useQuery } from "@tanstack/react-query"

import { readProbe } from "../lib/read-probe"
import { mediaSeekKeys } from "./media-seek-keys"
import { probeMedia } from "./probe-api"

export function useMediaProbe(toolId: string, path: string) {
  return useQuery({
    queryKey: mediaSeekKeys.probe(toolId, path),
    queryFn: () => probeMedia(toolId, path),
    select: readProbe,
    staleTime: Infinity,
    retry: false,
  })
}
