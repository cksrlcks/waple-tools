import { invoke } from "@tauri-apps/api/core"

import type { Collection, IconSet, InstalledSet } from "../model/types"

export function listIconSets(): Promise<InstalledSet[]> {
  return invoke("icon_sets")
}

export function installIconSet(prefix: string): Promise<InstalledSet> {
  return invoke("install_icon_set", { prefix })
}

export function checkIconUpdates(): Promise<InstalledSet[]> {
  return invoke("check_icon_updates")
}

export function removeIconSet(prefix: string): Promise<void> {
  return invoke("remove_icon_set", { prefix })
}

export function readIconSet(prefix: string): Promise<IconSet> {
  return invoke("read_icon_set", { prefix })
}

export function iconCollections(): Promise<Collection[]> {
  return invoke("icon_collections")
}
