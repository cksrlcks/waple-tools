import { invoke } from "@tauri-apps/api/core"

/** 쪽마다 썸네일 data URL (순서대로). 도구가 `x-page-picker`를 선언해야 한다 */
export function pageThumbnails(
  toolId: string,
  input: string,
): Promise<string[]> {
  return invoke("page_thumbnails", { toolId, input })
}
