import { useCallback, useState } from "react"

import { PAGE_SIZE } from "../config/icon-library"

/** 처음엔 PAGE_SIZE개, more()마다 PAGE_SIZE개씩 더. 검색·세트·스타일이 바뀌면 부모가 key로 새로 마운트한다 */
export function usePagedCount(total: number) {
  const [pages, setPages] = useState(1)
  const more = useCallback(() => setPages((p) => p + 1), [])
  return { count: Math.min(total, pages * PAGE_SIZE), more }
}
