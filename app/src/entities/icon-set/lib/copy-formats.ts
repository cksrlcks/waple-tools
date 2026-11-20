import { RECOMMENDED } from "../config/recommended"

export function toJsx(svg: string): string {
  return svg.replace(/\s([a-z]+(?:-[a-z]+)+)=/g, (all, attr: string) =>
    attr.startsWith("data-") || attr.startsWith("aria-")
      ? all
      : ` ${attr.replace(/-([a-z])/g, (_, c: string) => c.toUpperCase())}=`,
  )
}

export function toDataUri(svg: string): string {
  return `url("data:image/svg+xml,${encodeURIComponent(svg)}")`
}

export function toClassSnippet(prefix: string, name: string): string | null {
  const cls = RECOMMENDED.find((r) => r.prefix === prefix)?.className?.(name)
  return cls ? `<i class="${cls}"></i>` : null
}
