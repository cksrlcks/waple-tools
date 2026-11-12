import { invoke } from "@tauri-apps/api/core"

import type { PathStat } from "@/entities/tool"

/**
 * 넣을 파일을 도구에 `{"x-check-input": true}`로 물어 받을 수 있는 것만 남긴다 (`x-check-input` 도구).
 * 거절된 파일은 도구의 오류 문구와 함께 돌려준다. 폴더는 묻지 않는다.
 */
export async function checkInputs(
  toolId: string,
  items: PathStat[],
): Promise<{ ok: PathStat[]; rejected: string[] }> {
  const checks = await Promise.all(
    items.map((item) =>
      item.isDir
        ? null
        : invoke("run_interactive", {
            toolId,
            input: item.path,
            opts: { "x-check-input": true },
          }).then(
            () => null,
            (e: unknown) => (typeof e === "string" ? e : "확인할 수 없습니다."),
          ),
    ),
  )
  return {
    ok: items.filter((_, i) => checks[i] === null),
    rejected: checks.filter((c): c is string => c !== null),
  }
}
