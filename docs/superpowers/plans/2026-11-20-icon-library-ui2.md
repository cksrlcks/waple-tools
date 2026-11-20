# 아이콘 라이브러리 화면 수정 (설계 11장) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** 설계 11장의 화면 수정을 공용 부품 + DLL 변경으로 반영한다.

**Spec:** [docs/superpowers/specs/2026-11-20-icon-library-design.md](../specs/2026-11-20-icon-library-design.md) §11 (우선), 이전 장

## Global Constraints

- 2판 계획(docs/superpowers/plans/2026-11-20-icon-library.md)의 Global Constraints 그대로 (앱에 아이콘 개념 없음, 네트워크는 클릭 때만, 토큰만, 한국어, 한 파일 = 한 컴포넌트, ABI 그대로).
- 기존 도구 화면은 바뀌지 않는다 (새 플래그가 없으면 이전 동작).

---

### Task 1: core `actions` 블록 (Rust + TS)

**Files:** `crates/core/src/tool.rs`, `crates/core/src/lib.rs`(재내보내기), `app/src/entities/tool/model/types.ts`, `app/src/entities/tool/index.ts`

**Interfaces — Produces:**
```rust
Actions { items: Vec<ActionItem> },
// ActionItem { label: String, #[serde(flatten)] action: Action }
// Action (tag = "kind", camelCase): CopyText { text }, CopyImage { src /* data URL */ }, SaveText { text, name }
```
TS: `{ type: "actions"; items: ({ label: string } & ({ kind: "copyText"; text: string } | { kind: "copyImage"; src: string } | { kind: "saveText"; text: string; name: string }))[] }`

- [ ] 테스트: `actions_round_trip` — 세 kind 직렬화(`v["items"][1]["kind"] == "copyImage"`) + 역직렬화 왕복.
- [ ] 구현 → fmt·clippy·`cargo test --workspace`·`tsc` → 커밋 `feat: 결과 블록 actions(버튼 묶음) 추가`

### Task 2: DLL 옵션·블록 변경

**Files:** `crates/tools/web-icon-library/src/{lib.rs,blocks.rs,tests.rs}`, `Cargo.toml`(버전 그대로; 패키지 버전은 워크스페이스), 필요시 `png.rs`

**Interfaces:**
- 옵션: `size: u32` 기본 24, 범위 8~512 (null 아님); `stroke: Option<f64>` 0.5~3, `x-step` 0.25 (그대로); `color: String` 기본 `"#000000"`, `format: color`, `x-swatches: ["#000000","#6B7280","#2563EB","#DC2626"]`; `useColor`·`pngScale` 삭제.
- 스키마 루트: `x-panel-position: "bottom"`, `x-panel-blocks: ["actions"]` (나머지 플래그 그대로).
- 패널 블록: `Actions` 하나 — 순서 [SVG 복사(copyText svg)] [PNG 복사(copyImage, size px 1배)] [JSX 복사] [Data URI 복사] [클래스 복사(지원 세트만)] [SVG 저장(saveText, `<이름>.svg`)]. 색이 `#rrggbb`가 아니면 지금처럼 text 블록.
- 셀 svg는 그대로(24px, currentColor — 격자는 테마 글자색).
- [ ] 테스트 갱신: `schema_flags`(새 플래그, 없어진 옵션), `selected_produces_panel_blocks` → actions 하나·라벨 순서·kind, `png_size_scales` → size 48 → PNG 48×48, 색 기본 `#000000`이 svg에 들어감.
- [ ] 구현 → fmt·clippy·`cargo test --workspace` + `cargo rustc -p waple-tools-tool-web-icon-library --features addon --crate-type cdylib` → 커밋

### Task 3: 공용 옵션 칸 — 원래대로 슬라이더 · 색 견본

**Files:** `app/src/features/tool-options/lib/schema-to-fields.ts`(+test), `app/src/features/tool-options/ui/option-field.tsx`, 필요하면 새 컴포넌트 파일 (`ui/swatch-field.tsx` 등, 한 파일 한 컴포넌트)

**Interfaces:**
- null 허용 숫자 + min·max → `kind: "slider"`, `nullable: true`, `default: null` (2판 판정 "숫자 입력칸"을 대체). null이면 값 배지에 "원래", 썸은 `min` 위치; 옆 [원래대로] 버튼(값이 null이 아닐 때만 활성) → null.
- `format: color` + `x-swatches: string[]` → `kind: "swatches"`, `swatches`, `default`: 견본 버튼 4개 + 사용자 지정 1칸(색 입력; 견본에 없는 값이면 이 칸이 선택 상태). 고른 칸은 링/테두리 강조, `aria-pressed`.
- [ ] 테스트: schema-to-fields — nullable+range → slider nullable default null; x-swatches → swatches field; 기존 slider/number/color 결과 불변.
- [ ] 구현 → `npm test`·`tsc` → 커밋

### Task 4: 공용 화면 — actions 블록 · 목록 탭 · 아래 패널

**Files:** `app/src/features/interactive-result/ui/actions-block.tsx`(새), `block-list.tsx`, `resource-list-block.tsx`(+ 탭), `app/src/features/tool-options/lib/schema-flags.ts`(+`panelPosition(schema): "bottom" | "right"`, test), `app/src/widgets/tool-workspace/ui/interactive-tool-workspace.tsx`(+ 필요한 lib/ui 파일)

**Interfaces / 동작:**
- `ActionsBlock`: 버튼 한 줄(줄바꿈 허용). copyText → `navigator.clipboard.writeText` + toast "복사했습니다."; copyImage → data URL → Blob → `ClipboardItem` (기존 image-block 방식); saveText → 기존 저장 대화상자 흐름(`features/interactive-result/model/save-with-dialog.ts`, base64 false). 실패 toast "복사할 수 없습니다.".
- `ResourceListBlock` 탭 [설치됨 | 전체] (Segmented 또는 탭 UI, 토큰만): 설치됨 = 받은 리소스만. 처음 탭: 받은 게 있으면 설치됨, 없으면 전체. 검색은 고른 탭 안에서. 설치됨이 비면 "받은 세트가 없습니다." 안내 (항목·세트 같은 말은 공용 문구로: "받은 항목이 없습니다.").
- 아래 패널(`panelPosition === "bottom"`): 작업대 영역 안에서 아래에 붙어 격자 위를 덮는 패널(absolute, 그림자·위쪽 둥근 모서리, 최대 높이 제한 + 내부 스크롤, 테마 바탕 토큰). 내용 = 도구 이름 없이 옵션 폼(visibleFields) + 패널 블록. 하단 [닫기] → `panelWhen` 옵션을 ""로 (applyOption). `panelShown`이 거짓이면 그리지 않음. 오른쪽 aside는 그리지 않음. 올라오는 짧은 전환(기존 tw-animate-css 유틸 있으면 사용).
- [ ] 테스트: `panelPosition` 플래그; actions 블록의 순수 부분(data URL → Blob 변환 등)이 있으면 lib로 빼서 테스트; 탭 거르기 순수 함수 `filterByTab(items, installedIds, tab)` 테스트.
- [ ] 구현 → `npm test`·`tsc`·`npm run build` → 커밋

### Task 5: 확인 (컨트롤러)

- [ ] fmt·clippy·`cargo test --workspace`·`npm test`·`tsc`.
- [ ] DLL 릴리스 빌드 → 로컬 index 버전 0.1.1로 서명·갱신 → 앱 실행 → 사이드바 "업데이트" → 업데이트 흐름 확인 → 사용자 수동 확인.
