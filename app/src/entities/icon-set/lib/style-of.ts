import { RECOMMENDED } from "../config/recommended"
import type { IconSet } from "../model/types"

export function stylesOf(set: IconSet): Record<string, string> {
  return RECOMMENDED.find((r) => r.prefix === set.prefix)?.styles ?? set.suffixes ?? {}
}

/** 이름 끝이 `-키`인 가장 긴 키. 없으면 "" */
export function styleOf(name: string, styles: Record<string, string>): string {
  let best = ""
  for (const key of Object.keys(styles)) {
    if (key.length > best.length && name.endsWith("-" + key)) best = key
  }
  return best
}
