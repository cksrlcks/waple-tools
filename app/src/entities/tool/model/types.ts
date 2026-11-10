export type ToolKind = "perFile" | "combine" | "generator" | "interactive"

export type ToolStatus = "builtin" | "notInstalled"

/** 카탈로그 항목 + 설치 상태. 설치된 도구만 kind·옵션 스키마·확장자가 있다. */
export type ToolEntry = {
  id: string
  name: string
  category: string
  description: string
  status: ToolStatus
  kind: ToolKind | null
  optionsSchema: unknown
  extensions: string[]
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

export type PathStat = {
  path: string
  /** 폴더는 0 */
  size: number
  isDir: boolean
  exists: boolean
}

export type PlannedFile = {
  input: string
  root: string
  rel: string
  destDir: string
}

export type Plan = {
  files: PlannedFile[]
  skipped: Skipped[]
}

/** 화면용 이미지 (data URL) */
export type Shown = {
  url: string
  /** 원본 크기 */
  width: number
  height: number
}

/** 원본 ↔ 결과 비교 */
export type ToolPreview = {
  before: Shown
  /** 결과가 이미지가 아니면 없음 */
  after: Shown | null
  sizeBefore: number
  sizeAfter: number | null
  /** 도구가 실패한 이유 (원본은 그대로 보여 준다) */
  error: string | null
}
