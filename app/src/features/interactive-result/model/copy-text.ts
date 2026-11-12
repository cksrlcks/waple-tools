import { toast } from "sonner"

/** 클립보드에 글을 넣고 결과를 toast로 알린다 */
export function copyText(text: string) {
  navigator.clipboard
    .writeText(text)
    .then(() => toast.success("복사했습니다."))
    .catch(() => toast.error("복사할 수 없습니다."))
}
