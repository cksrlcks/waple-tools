import type { Block } from "@/entities/tool"
import type { Field } from "@/features/tool-options"

/** `x-fill-from` 옵션: [옵션 키, 채울 글이 든 코드 블록 이름(lang)] */
export type FillTarget = readonly [key: string, lang: string]

export function fillTargets(fields: Field[]): FillTarget[] {
  return fields.flatMap((f) =>
    f.kind === "multiline" && f.fillFrom ? [[f.key, f.fillFrom] as const] : [],
  )
}

/** 비어 있는 칸만, 결과에 같은 이름의 코드 블록이 있을 때 그 글로 채운다 */
export function fillPatch(
  targets: FillTarget[],
  blocks: Block[],
  options: Record<string, unknown>,
): Record<string, string> {
  const patch: Record<string, string> = {}
  for (const [key, lang] of targets) {
    const code = blocks.find((b) => b.type === "code" && b.lang === lang)
    if (code?.type === "code" && !options[key]) patch[key] = code.code
  }
  return patch
}

/** 칸을 채우는 코드 블록은 결과에서 뺀다 (칸과 같은 내용) */
export function hideFilled(blocks: Block[], targets: FillTarget[]): Block[] {
  return blocks.filter(
    (b) => !(b.type === "code" && targets.some(([, lang]) => lang === b.lang)),
  )
}
