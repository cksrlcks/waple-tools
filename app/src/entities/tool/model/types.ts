export type ToolKind =
  | "perFile"
  | "combine"
  | "generator"
  | "interactive"
  /** 파일명 일괄 변경: 원본 이름을 제자리에서 바꾼다 (미리보기 표 → 실행 → 되돌리기) */
  | "rename"

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
  /** AI 모델을 쓰는 도구 (목록에 "AI" 뱃지) */
  ai?: boolean
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

/** 파일명 일괄 변경 미리보기 한 줄. `name`은 도구가 정한 새 이름 그대로 (쓸 수 없는 글자 포함) */
export type RenameRow = {
  from: string
  to: string
  name: string
} & (
  | { status: "changed" }
  | { status: "unchanged" }
  | { status: "conflict"; reason: string }
)

export type RenamePlan = {
  rows: RenameRow[]
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
  /** 원본을 그림으로 보일 수 없으면(예: 영상) 없음 */
  before: Shown | null
  /** 결과가 이미지가 아니면 없음 */
  after: Shown | null
  sizeBefore: number
  sizeAfter: number | null
  /** 도구가 실패한 이유 (원본은 그대로 보여 준다) */
  error: string | null
  /** `x-preview-crop`: 원본·결과 모두 가운데 조각 */
  cropped: boolean
}

/** Interactive 도구의 결과 블록. 앱은 블록 종류별 뷰어만 갖는다 */
export type Block =
  | { type: "text"; text: string }
  /** `name`이 있으면 [저장] 버튼, 기본 파일 이름 */
  | { type: "code"; lang: string; code: string; name?: string }
  | { type: "number"; label: string; value: number; unit: string | null }
  /** `label`: 칩 이름(예: `Vibrant · 32.4%`), `text`: 칩 위 글자색, `detail`: `rgb(…) · hsl(…)` */
  | {
      type: "color"
      hex: string
      label?: string
      text?: string
      detail?: string
    }
  /** `src`는 data URL. [이미지 복사]·[저장] */
  | { type: "image"; src: string; name: string }
  | { type: "imagePair"; a: string; b: string; diff: string }
  /** 강조 구간이 있는 글 (정규식 일치). 조각을 이으면 원문 */
  | { type: "highlight"; spans: Span[] }
  /** 표 + [복사](탭 구분). `name`이 있으면 [저장](CSV) */
  | { type: "table"; columns: string[]; rows: string[][]; name?: string }
  /** 텍스트 비교. `split`은 이어진 delete·insert 묶음을 한 줄씩 짝지어 좌우로 그린다 */
  | { type: "diff"; layout: "split" | "unified"; lines: DiffLine[] }
  /** 카드 목록 + [복사] (정규식 일치 하나 = 카드 하나) */
  | { type: "records"; items: RecordItem[] }

/** `records` 블록의 카드: 이름표(`매치 1`), 보조 글(`위치 13–29`), 본문, 이름·값 줄 */
export type RecordItem = {
  title: string
  meta?: string
  text: string
  fields?: { label: string; value: string }[]
}

/** `mark`가 있으면 강조: 비교는 줄 안에서 바뀐 글자(0), 정규식은 몇 번째 일치(0부터, 짝·홀로 색을 번갈아) */
export type Span = { text: string; mark?: number }

export type DiffLine = {
  tag: "equal" | "delete" | "insert"
  /** 원본 줄 번호 (1부터). insert는 없음 */
  old?: number
  /** 비교 대상 줄 번호 (1부터). delete는 없음 */
  new?: number
  spans: Span[]
}
