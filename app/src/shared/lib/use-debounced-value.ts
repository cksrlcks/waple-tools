import { useEffect, useState } from "react"

/** 값이 `ms` 동안 바뀌지 않으면 그 값을 돌려준다 */
export function useDebouncedValue<T>(value: T, ms: number): T {
  const [debounced, setDebounced] = useState(value)
  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), ms)
    return () => clearTimeout(id)
  }, [value, ms])
  return debounced
}
