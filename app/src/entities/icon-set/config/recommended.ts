export type Recommended = {
  prefix: string
  name: string
  license: string
  styles?: Record<string, string>
  className?: (name: string) => string
}

export const RECOMMENDED: Recommended[] = [
  {
    prefix: "tabler",
    name: "Tabler Icons",
    license: "MIT",
    styles: { "": "선", filled: "채움" },
    className: (n) => `ti ti-${n}`,
  },
  {
    prefix: "heroicons",
    name: "Heroicons",
    license: "MIT",
    styles: { "": "선 24", solid: "채움 24", "20-solid": "채움 20", "16-solid": "채움 16" },
  },
  {
    prefix: "ri",
    name: "Remix Icon",
    license: "Apache-2.0",
    styles: { line: "선", fill: "채움" },
    className: (n) => `ri-${n}`,
  },
]
