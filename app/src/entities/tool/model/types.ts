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

export type OutputPolicy =
  | { type: "InPlaceWithBackup" }
  | { type: "KeepOriginal"; suffix: string }
  | { type: "OutputFolder"; dir: string }

export type JobSpec = {
  inputs: string[]
  recursive: boolean
  policy: OutputPolicy
  keepIfLarger: boolean
  /** 0이면 CPU 코어 수 - 1 */
  threads: number
  opts: Record<string, unknown>
}

export type Progress = {
  done: number
  total: number
}

export type FileResult =
  | {
      status: "done"
      input: string
      outputs: string[]
      backup: string | null
      sizeBefore: number
      sizeAfter: number
    }
  | { status: "skipped"; input: string; reason: string }
  | { status: "failed"; input: string; message: string }
  | { status: "cancelled"; input: string }

export type Skipped = {
  path: string
  reason: string
}

export type JobReport = {
  jobId: string
  files: FileResult[]
  skipped: Skipped[]
}

export type UndoItem =
  | { status: "restored"; path: string }
  | { status: "removed"; path: string }
  | { status: "skipped"; path: string; reason: string }
