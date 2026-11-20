import { useCallback, useRef, useState } from "react"

import {
  iconSize,
  RECOMMENDED,
  strokeOf,
  stylesOf,
  useIconSet,
  useInstalledSets,
  type IconItem,
} from "@/entities/icon-set"

/**
 * 세트·스타일·검색·선택 아이콘·크기·두께·색.
 * 스타일·선택은 고른 세트에 묶어 둔다: 세트가 바뀌면(삭제로 바뀌어도) 선택은 없고 스타일은 첫 키.
 */
export function useIconLibraryState() {
  const installed = useInstalledSets()
  const [chosen, setPrefix] = useState<string | null>(null)
  const [styleBy, setStyleBy] = useState<{
    prefix: string
    style: string | null
  } | null>(null)
  const [query, setQuery] = useState("")
  const [selectedBy, setSelectedBy] = useState<{
    prefix: string
    icon: IconItem
  } | null>(null)
  const [size, setSize] = useState(24)
  const [stroke, setStroke] = useState<number | null>(null)
  const [color, setColor] = useState("currentColor")

  const sets = installed.data
  const known = (p: string) =>
    !!sets?.some((s) => s.prefix === p) ||
    RECOMMENDED.some((r) => r.prefix === p)
  // 받은 세트 목록을 읽기 전에는 세트를 정하지 않는다. 고른 세트가 목록에서 사라지면 처음 세트로
  const prefix = !sets
    ? null
    : chosen && known(chosen)
      ? chosen
      : (sets[0]?.prefix ?? RECOMMENDED[0].prefix)
  const isInstalled = !!prefix && !!sets?.some((s) => s.prefix === prefix)
  const iconSet = useIconSet(isInstalled ? prefix : null)
  const set = isInstalled ? iconSet.data : undefined

  // 스타일 키가 1개 이하면 고를 것이 없어(Segmented도 숨김) 거르지 않는다
  const styleKeys = set ? Object.keys(stylesOf(set)) : []
  const style =
    styleKeys.length <= 1
      ? null
      : styleBy && styleBy.prefix === prefix
        ? styleBy.style
        : styleKeys[0]
  const setStyle = (s: string | null) =>
    prefix && setStyleBy({ prefix, style: s })
  const selected =
    selectedBy && selectedBy.prefix === prefix ? selectedBy.icon : null

  // 콜백을 고정해 두고도 현재 선택을 보려고 ref에 둔다
  const selectedName = useRef<string | undefined>(undefined)
  selectedName.current = selected?.name

  // 격자 셀 memo가 유지되도록 고정 (세트가 바뀔 때만 다시 만든다)
  const setSelected = useCallback(
    (icon: IconItem | null) => {
      // 선택된 아이콘을 다시 누르면 선택을 푼다
      if (!icon || !set || icon.name === selectedName.current)
        return setSelectedBy(null)
      setSelectedBy({ prefix: set.prefix, icon })
      setSize(iconSize(icon, set).height)
      setStroke(strokeOf(icon.body))
    },
    [set],
  )

  return {
    installed: sets,
    iconSet,
    set,
    isInstalled,
    prefix,
    setPrefix,
    style,
    setStyle,
    query,
    setQuery,
    selected,
    setSelected,
    size,
    setSize,
    stroke,
    setStroke,
    color,
    setColor,
  }
}
