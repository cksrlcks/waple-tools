/** `from` 자리 항목을 `to` 자리로 옮긴 새 배열. 범위 밖이면 끝으로 붙이고, 같으면 그대로 */
export function moveItem<T>(list: T[], from: number, to: number): T[] {
  if (from < 0 || from >= list.length) return list
  const target = Math.max(0, Math.min(list.length - 1, to))
  if (target === from) return list
  const next = [...list]
  const [item] = next.splice(from, 1)
  next.splice(target, 0, item)
  return next
}
