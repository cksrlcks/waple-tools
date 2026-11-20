import { invoke } from "@tauri-apps/api/core"
import { save } from "@tauri-apps/plugin-dialog"
import { toast } from "sonner"

import { baseName } from "@/shared/lib/base-name"
import { toUserMessage } from "@/shared/lib/to-user-message"

/** 저장 창에서 위치를 고르고 새 파일로 저장한다 (덮어쓰지 않음). 결과는 toast로 알린다 */
export async function saveSvg(name: string, svg: string) {
  try {
    const path = await save({
      defaultPath: `${name}.svg`,
      filters: [{ name: "SVG", extensions: ["svg"] }],
    })
    if (!path) return
    const saved = await invoke<string>("save_block", {
      path,
      content: svg,
      base64: false,
    })
    toast.success(`저장했습니다: ${baseName(saved)}`)
  } catch (err) {
    toast.error(toUserMessage(err))
  }
}
