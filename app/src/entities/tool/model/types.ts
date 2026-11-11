export type ToolKind = "perFile" | "combine" | "generator" | "interactive"

export type ToolStatus =
  | "builtin"
  /** 아직 만들지 않은 도구 (메뉴에 "준비 중"으로만 보인다) */
  | "planned"
  | "installed"
  | "notInstalled"
  /** 같은 앱에서 쓸 수 있는 새 버전이 있다 */
  | "updateAvailable"
  /** 설치됐지만 읽을 수 없다 (앱이 바뀌었거나 파일이 바뀜) — 다시 설치해야 쓸 수 있다 */
  | "updateRequired"

/** 카탈로그 항목 + 설치 상태. 쓸 수 있는 도구만 kind·옵션 스키마·확장자가 있다. */
export type ToolEntry = {
  id: string
  name: string
  category: string
  description: string
  status: ToolStatus
  kind: ToolKind | null
  optionsSchema: unknown
  extensions: string[]
  /** 추가 도구: 설치된 버전 */
  version: string | null
  /** 추가 도구: 이 앱용 최신 버전과 다운로드 크기 (목록을 못 받았으면 없음) */
  latest: string | null
  size: number | null
}

export type DownloadProgress = {
  received: number
  total: number | null
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
  /** 0~1 전체 비율 (진행 중인 파일의 도구 진행률 포함) */
  ratio: number
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

/** Interactive 도구의 결과 블록. 앱은 블록 종류별 뷰어만 갖는다 */
export type Block =
  | { type: "text"; text: string }
  | { type: "code"; lang: string; code: string }
  | { type: "number"; label: string; value: number; unit: string | null }
  | { type: "color"; hex: string }
  | { type: "image"; path: string }
  | { type: "imagePair"; a: string; b: string; diff: string }
