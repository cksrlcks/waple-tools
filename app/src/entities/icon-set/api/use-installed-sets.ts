import { useQuery } from "@tanstack/react-query"

import { listIconSets } from "./icon-set-api"
import { iconSetKeys } from "./icon-set-keys"

export function useInstalledSets() {
  return useQuery({ queryKey: iconSetKeys.installed(), queryFn: listIconSets })
}
