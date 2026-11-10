export type ToolKind = "perFile" | "combine" | "generator" | "interactive"

export type ToolMeta = {
  id: string
  name: string
  category: string
  description: string
  extensions: string[]
}

export type ToolInfo = {
  meta: ToolMeta
  kind: ToolKind
  optionsSchema: unknown
}

export type Progress = {
  fileIndex: number
  total: number
  ratio: number
}

export type FileResult =
  | { status: "done"; input: string; outputs: string[] }
  | { status: "failed"; input: string; message: string }
  | { status: "cancelled"; input: string }

export type RunResult = {
  jobId: string
  files: FileResult[]
}
