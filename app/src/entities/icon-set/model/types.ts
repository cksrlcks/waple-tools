export type IconItem = {
  name: string
  body: string
  width?: number | null
  height?: number | null
}

export type IconSet = {
  prefix: string
  name: string
  version: string
  license: { title: string; spdx: string | null }
  palette: boolean
  width: number
  height: number
  suffixes: Record<string, string>
  icons: IconItem[]
}

export type InstalledSet = {
  prefix: string
  name: string
  version: string
  latest: string | null
}

export type Collection = {
  prefix: string
  name: string
  total: number
  category: string | null
  license: string
  palette: boolean
}
