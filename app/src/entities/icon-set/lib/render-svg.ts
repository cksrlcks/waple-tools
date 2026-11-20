import type { IconItem, IconSet } from "../model/types"

export type RenderOpts = { size: number; stroke: number | null; color: string }

export function iconSize(icon: IconItem, set: IconSet): { width: number; height: number } {
  return { width: icon.width ?? set.width, height: icon.height ?? set.height }
}

export function strokeOf(body: string): number | null {
  const m = /stroke-width="([\d.]+)"/.exec(body)
  return m ? Number(m[1]) : null
}

export function toSvg(icon: IconItem, set: IconSet, opts: RenderOpts): string {
  const { width: w, height: h } = iconSize(icon, set)
  let body = icon.body
  if (opts.stroke != null) body = body.replace(/stroke-width="[^"]*"/g, `stroke-width="${opts.stroke}"`)
  if (opts.color !== "currentColor") body = body.replace(/currentColor/g, opts.color)
  const width = Math.round((opts.size * w) / h)
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${opts.size}" viewBox="0 0 ${w} ${h}">${body}</svg>`
}
