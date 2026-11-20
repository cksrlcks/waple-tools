import { invoke } from "@tauri-apps/api/core"

import type { ResourceState } from "../model/types"

export const toolResources = (toolId: string): Promise<ResourceState[]> =>
  invoke("tool_resources", { toolId })

export const installToolResource = (
  toolId: string,
  id: string,
): Promise<ResourceState> => invoke("install_tool_resource", { toolId, id })

export const checkToolResources = (toolId: string): Promise<ResourceState[]> =>
  invoke("check_tool_resources", { toolId })

export const removeToolResource = (toolId: string, id: string): Promise<void> =>
  invoke("remove_tool_resource", { toolId, id })
