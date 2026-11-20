import { toast } from "sonner"

import { dataUrlBlob } from "../lib/data-url-blob"

/** 클립보드에 그림(data URL)을 넣고 결과를 toast로 알린다 (data URL 풀기 실패도 실패 toast) */
export function copyImage(src: string) {
  Promise.resolve()
    .then(() => {
      const blob = dataUrlBlob(src)
      return navigator.clipboard.write([
        new ClipboardItem({ [blob.type]: blob }),
      ])
    })
    .then(() => toast.success("이미지를 복사했습니다."))
    .catch(() => toast.error("복사할 수 없습니다."))
}
