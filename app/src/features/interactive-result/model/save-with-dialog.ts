import { save } from "@tauri-apps/plugin-dialog"
import { toast } from "sonner"

import { baseName } from "@/shared/lib/base-name"
import { toUserMessage } from "@/shared/lib/to-user-message"

import { saveBlock } from "../api/interactive-api"

/** 저장 창에서 위치를 고르고 새 파일로 저장한다 (덮어쓰지 않음). 결과는 toast로 알린다 */
export async function saveWithDialog(
  name: string,
  content: string,
  base64: boolean,
) {
  const ext = name.includes(".") ? name.split(".").pop() : undefined
  try {
    const path = await save({
      defaultPath: name,
      filters: ext ? [{ name: ext.toUpperCase(), extensions: [ext] }] : [],
    })
    if (!path) return
    const saved = await saveBlock(path, content, base64)
    toast.success(`저장했습니다: ${baseName(saved)}`)
  } catch (err) {
    toast.error(toUserMessage(err))
  }
}
