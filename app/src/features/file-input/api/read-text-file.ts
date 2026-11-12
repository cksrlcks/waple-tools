import { invoke } from "@tauri-apps/api/core"

/** UTF-8 글 파일 내용 (20MB까지). 글이 아니거나 너무 크면 한국어 문구로 실패한다 */
export function readTextFile(path: string): Promise<string> {
  return invoke("read_text_file", { path })
}
