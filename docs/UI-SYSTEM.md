# U-Drive UI 시스템 이식 가이드

## 0. waple tools 적용 (이 문서보다 우선)

아래 U-Drive 문서를 바탕으로 하되, waple tools는 색과 화면 틀을 바꿨다. 값은 `app/src/app/styles/index.css`가 기준이다.

- **색**: Windows 제목 막대 회색(`--titlebar` `#f3f3f3`, 다크 `#202020`) + 흰 본문(`--background`) + 짙은 남색 강조 하나(`--primary` `#1e40af`, 다크 `#2563eb`). 선택 행은 `--selection`. 강조색은 탭마다 다르다(`<html data-category>`): 이미지 남색, PDF 빨강(Acrobat 계열), 영상 주황(호박색), 웹폰트 보라, 기타 청록, 개발용 슬레이트.
- **작업대 토큰**: `--stage` · `--stage-foreground` · `--stage-muted`. 테마와 상관없이 어두운 미리보기 바탕(`bg-stage`)과 그 위 글자. 작업대 위의 선·칸은 `stage-foreground/8~20` 투명도로 만든다. 글 편집기 도구(`x-multiline`)는 작업대에 `.stage-plain`을 붙여 작업대 토큰을 테마 바탕으로 바꾼다(편집기·입력칸은 흰 `bg-card` + 테두리).
- **버튼**: 아래쪽만 진한 Windows식 테두리를 없앴다(`--*-edge`가 바탕과 같은 색). 크기 규칙은 그대로.
- **폰트**: 시스템 폰트만 쓴다. 영문·숫자는 Segoe UI Variable, 한글은 맑은 고딕(Segoe UI에 한글이 없어 다음 글꼴로 넘어감). 웹 폰트를 묶지 않는다.
- **창 틀**: 기본 제목 표시줄을 끄고(`decorations: false`) 앱이 그린다. `widgets/title-bar` = 로고 · 메뉴(파일·보기·도움말) · 카테고리 탭 · 창 버튼. 빈 곳은 `data-tauri-drag-region`. 설정은 탭이 아니라 파일 › 설정… 대화상자.
- **작업 화면 3단**: 도구 목록(`w-56`) | 작업대(`bg-stage rounded-xl`, 위는 미리보기, 아래는 넣은 파일 필름 띠) | 옵션 패널(`w-80`, 위는 스크롤, 실행 막대는 아래 고정).
- **옵션 패널**: 묶음 사이는 가는 선(`divide-y`), 묶음 제목은 `text-xs font-semibold`, 묶음 스위치는 `Switch`(꺼지면 칸을 접음). 칸은 "이름 왼쪽 · 입력 오른쪽" 한 줄.
- 대문자 라벨·카드 그림자 남발 금지. 칸은 선으로 나누고 강조색은 하나만.

> 기준: U-Drive v0.16.0 (2026-11-10). 다른 React + Tailwind v4 프로젝트에 이 UI를 그대로 가져갈 때 읽는 문서다.
> `docs/DESIGN.md`는 일부 값(Inter 폰트, 회색 캔버스, 15px 본문)이 예전 것이다. 값이 다르면 이 문서와 `src/index.css`가 맞다.

## 1. 개요

U-Drive UI는 **웹 페이지가 아니라 Windows 11·macOS 데스크톱 프로그램처럼 보이는** shadcn 기반 시스템이다. 색·그림자·크기는 `src/index.css` 토큰 한 곳에만 있고, 기본 요소는 `src/components/ui/` 12개 파일이 전부다.

| 항목 | 사용 기술 |
| --- | --- |
| 프레임워크 | React 19 + Vite 8 (Tauri 2 데스크톱 셸이지만 UI는 Tauri 없이도 동작) |
| 스타일 | Tailwind CSS v4 (`@theme inline`, 설정 파일 없음) + `tw-animate-css` |
| 컴포넌트 | shadcn `base-nova` 스타일 — Radix가 아니라 `@base-ui/react` 프리미티브 |
| 클래스 병합 | `cn` 패키지 (`src/lib/utils.ts`가 다시 내보냄) |
| 아이콘 | `components/ui` 안은 `lucide-react`(shadcn 기본), 화면 코드는 `@tabler/icons-react` (`size-4`, `stroke={1.75}`) |
| 폰트 | 시스템 UI 폰트 우선(Segoe UI Variable / SF Pro) → `Pretendard Variable` 대체 |

핵심 원칙:

1. **토큰이 유일한 소스.** 색·반경·그림자는 `index.css`의 CSS 변수에만 쓴다. 화면 코드에 `#hex`, `bg-[…]`, `text-gray-500` 금지.
2. **데스크톱 밀도.** 본문 13px(`text-sm`), 버튼 26px, 메뉴 항목 24px, 표 행 34px.
3. **기본 요소는 `components/ui`를 거친다.** 화면 코드에서 맨 `<input>`·직접 만든 모달·메뉴를 쓰지 않는다.
4. **웹식 움직임 금지.** 확대(`zoom-*`)·미끄러짐(`slide-in-from-*`)·누를 때 내려가기(`translate-y-px`) 없이 75~100ms 페이드만.
5. **다크 모드는 `<html class="dark">` 하나로.** 모든 토큰이 `.dark`에서 다시 정의된다.

## 2. 디자인 토큰

`src/index.css` 한 파일에 세 층으로 있다: `:root`(라이트) → `.dark`(다크) → `@theme inline`(Tailwind 클래스로 노출). 새 토큰을 추가할 때도 세 곳을 모두 채운다.

### 색상 (라이트 기준)

| 토큰 | 값 | 쓰임 |
| --- | --- | --- |
| `--background` / `--card` | `#ffffff` | 본문·카드 바탕 |
| `--foreground` | `#1b1b1b` | 기본 글자 |
| `--primary` | `#0a63c9` | 유일한 포인트색 (다크 `#1f74db`) |
| `--muted` / `--muted-foreground` | `#f3f3f3` / `#6b6b6b` | 보조 바탕 / 보조 글자 |
| `--accent` / `--secondary` | `#e9e9e9` / `#ebebeb` | hover · 보조 버튼 |
| `--border` / `--input` | `#e3e3e3` / `#d6d6d6` | 구분선 / 입력 테두리 |
| `--destructive` / `--success` | `#c42b1c` / `#107c10` | 위험 / 성공 |
| `--sidebar` / `--titlebar` | `#f3f3f3` | 사이드바·타이틀바·상태 막대 연회색 |
| `--surface-header` | `#fafafa` | 표 머리줄 |
| `--selection` | `#d6e6fb` | 선택한 행 (Windows 11 탐색기 톤, 다크 `#264f78`) |
| `--folder-*` (5개) | `#8fcbf8` → `#4a9df0` | 입체 폴더 그림 |
| `--brand-a` / `--brand-b` | `#5cb5ff` / `#1d6fe0` | 로고 그라디언트 |

### 데스크톱 부품 전용 토큰

메뉴·대화상자·버튼의 "데스크톱 느낌"을 만드는 토큰이다. 라이트·다크 둘 다 있어야 하며 `src/components/ui/desktop-style.test.ts`가 검사한다.

- `--menu` `rgb(246 246 246 / .72)` + `--menu-separator` — 반투명 유리 메뉴
- `--dialog-footer` `#f5f5f5` + `--dialog-footer-border` `#d9d9d9` — 대화상자 아래 회색 띠
- `--primary-hover` / `--primary-active` / `--destructive-hover` / `--outline-hover` — 버튼 상태
- `--{primary,outline,destructive}-edge` + `-edge-bottom` — 버튼 테두리 (아래쪽만 한 단계 진하게)

### 글자 크기

Tailwind 기본 이름은 그대로 두고 값만 줄였다. 새 이름(`text-md` 등)을 만들면 tailwind-merge가 글자색으로 오인해 색 클래스를 지운다.

| 클래스 | 크기 / 줄 높이 | 쓰임 |
| --- | --- | --- |
| `text-base` | 14px / 1.4 | 대화상자 제목 |
| `text-sm` | 13px / 1.35 | UI 본문 (기본) |
| `text-xs` | 12px / 1.3 | 보조 · 단축키 · 그리드 라벨 |
| `text-2xs` | 11px / 1.3 | 표 머리글 · 섹션 라벨 · 상태 막대 |

자간: `body` -0.01em, `h1~h3` -0.02em.

### 반경 · 그림자

`--radius: 0.5rem`(8px)에서 파생: `sm` 4.8 · `md` 6.4 · `lg` 8 · `xl` 11.2 · `2xl` 14.4px. 버튼·입력·목록 항목 `rounded-md`, 대화상자 `rounded-xl`, 메뉴 10px.

| 클래스 | 쓰임 |
| --- | --- |
| `shadow-menu` | 메뉴 (0.5px 외곽선 포함 — ring 따로 안 씀) |
| `shadow-dialog` | 대화상자, 끌기 중 따라다니는 라벨 |
| `shadow-button` / `shadow-button-subtle` | 채운 버튼 / outline 버튼 |
| `shadow-float` | 카드 hover |
| `shadow-item` | 썸네일, 작은 배지 |

### 커스텀 variant

- `dark:` — `@custom-variant dark (&:is(.dark *))`
- `mac:` — `<html data-os="mac">`일 때만 적용되는 맥 보정. `main.tsx`에서 `if (isMac()) document.documentElement.dataset.os = "mac"`.

## 3. 기본 컴포넌트 (`src/components/ui/`)

대부분 shadcn `base-nova`로 받은 뒤 데스크톱 모양으로 고친 것이고, `cloud-mark`·`folder-glyph`·`menu-styles`는 직접 만든 것이다.

| 파일 | 내보내는 것 | shadcn 기본과 다른 점 |
| --- | --- | --- |
| `button.tsx` | `Button`, `buttonVariants` | 아래 표 참고 |
| `input.tsx` | `Input` | 높이 26px(`h-6.5`), `rounded-md`, 포커스 `ring-2 ring-primary/40` |
| `checkbox.tsx` | `Checkbox` | 16px, 클릭 영역을 `after:` 의사 요소로 넓힘 |
| `dialog.tsx` | `Dialog*` 10개 | 아래 회색 띠 footer, ✕ 버튼 기본 숨김, 페이드만 |
| `context-menu.tsx` | `ContextMenu*` 15개 | `menu-styles` 사용 |
| `dropdown-menu.tsx` | `DropdownMenu*` 15개 | `menu-styles` 사용 |
| `menubar.tsx` | `Menubar*` 16개 | `menu-styles` 사용 |
| `menu-styles.ts` | 메뉴 공용 클래스 문자열 7개 | 직접 작성 |
| `tooltip.tsx` | `Tooltip*` 4개 | `bg-foreground text-background` 반전형, 지연 0 |
| `progress.tsx` | `Progress*` 5개 | 트랙 4px(`h-1`) |
| `folder-glyph.tsx` | `FolderGlyph` | `--folder-*` 토큰으로 그린 입체 폴더 SVG |
| `cloud-mark.tsx` | `CloudMark` | `--brand-*` 그라디언트 구름 로고 |

### Button

테두리 있는 세 변형만 데스크톱 모양(26px, `rounded-md`, `font-normal`, 아래 테두리만 진하게)이다. 테두리 없는 변형은 툴바·선택 막대용이라 한 단계 크다.

| variant | 모양 | `default` / `sm` / `lg` 높이 |
| --- | --- | --- |
| `default` | 파랑 바탕 + `primary-edge` 테두리 + `shadow-button` | 26 / 24 / 28px |
| `outline` | 흰 바탕 + `outline-edge` 테두리 | 26 / 24 / 28px |
| `destructive` | 빨강 바탕 + 흰 글자 | 26 / 24 / 28px |
| `ghost` | 투명, hover `bg-muted` | 32 / 28 / 36px |
| `secondary` | `bg-secondary` | 32 / 28 / 36px |
| `link` | `text-primary` 밑줄 | 32 / 28 / 36px |

아이콘 버튼: `icon-xs` 24 · `icon-sm` 28 · `icon` 32 · `icon-lg` 36px. 기본 `leading-none` — 줄 높이가 글꼴보다 크면 글자가 아이콘보다 1px 위로 그려지기 때문.

### 메뉴 (`menu-styles.ts`)

우클릭·드롭다운·메뉴 막대가 같은 클래스 문자열을 import한다. 메뉴 모양을 바꾸려면 이 파일만 고친다.

- `menuPopup` — `bg-menu` + `backdrop-blur-[24px] backdrop-saturate-[1.8]` + `shadow-menu`, 반경 10px, 안쪽 5px, 최소 폭 144px
- `menuItem` — 높이 24px, 강조 시 중립 회색 `bg-foreground/8`(메뉴 막대 버튼 hover와 같은 색, 다크에서도 보이게 글자색 기준), `variant="destructive"`는 빨간 강조. 아이콘 있는 메뉴에서 아이콘 없는 항목은 `inset`(`pl-8`)으로 글자 시작선을 맞춘다
- `menuCheckItem` · `menuSubTrigger` · `menuSeparator` · `menuShortcut` · `menuLabel`

### Dialog

흰 본문 + 아래 회색 띠(`DialogFooter`, 항상 가로·오른쪽 정렬). 기본 폭 384px(`sm:max-w-sm`), 배경 `bg-black/18`. ✕는 footer가 없는 창에만 `showCloseButton`으로 켠다. 아이콘 장식은 넣지 않는다.

```tsx
<Dialog open={open} onOpenChange={setOpen}>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>폴더 삭제</DialogTitle>
      <DialogDescription>휴지통으로 이동합니다.</DialogDescription>
    </DialogHeader>
    <DialogFooter>
      <DialogClose render={<Button variant="outline" />}>취소</DialogClose>
      <Button variant="destructive" onClick={onDelete}>삭제</Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
```

Base UI는 Radix의 `asChild` 대신 `render` prop을 쓴다.

## 4. 드라이브 화면 패턴

UI 부품이 아니라 화면 조립 방식이다. `src/features/browse/`, `src/features/shell/`에 있다. 다른 프로젝트에서는 **복사하지 말고 클래스와 수치를 참고해서 다시 만든다** (저장소 API·Tauri에 묶여 있음).

### 창 레이아웃

```
┌──────────────────────────────────────────────────────────┐
│ TitleBar  bg-titlebar  border-b                          │
│  h-8  메뉴 막대 (MenuBar)                                │
│  h-9  ← → ↑ │ 경로 막대(h-7) │ 보기 전환 │ 새로고침 │ 업로드 │
├────────────┬┬──────────────────────────────┬─────────────┤
│ Sidebar    ││ main                         │ DetailsPanel│
│ bg-sidebar ││ 오류 배너 (bg-destructive/10)  │ w-70        │
│ border-r   ││ FileList 또는 IconGrid        │ bg-sidebar  │
│ 너비 조절   ││ (overflow-auto, 마퀴 선택)     │ border-l    │
│            ││                  알림 스택 ↘  │             │
├────────────┴┴──────────────────────────────┴─────────────┤
│ StatusBar  h-6  bg-sidebar  border-t  text-2xs           │
└──────────────────────────────────────────────────────────┘
```

- 바깥: `flex h-screen flex-col overflow-hidden bg-background text-sm text-foreground`
- 사이드바·표 열 경계의 너비 조절 손잡이: 4px 투명 영역 + 1px 선, hover·끄는 중엔 `bg-primary`
- 알림(업로드·다운로드·이동 되돌리기): `absolute right-3 bottom-3 flex flex-col items-end gap-2`

### 사이드바

- 컨테이너: `flex flex-col overflow-y-auto border-r border-border bg-sidebar p-2`
- 섹션 제목: `text-2xs font-medium text-muted-foreground`, 접기 화살표는 hover 때만 보임
- 행: `my-px flex h-8 items-center gap-2 rounded-md px-2 text-sm font-medium`
  - hover `bg-sidebar-accent`, 선택 `bg-selection` + 아이콘 `text-primary`, 평소 아이콘 `text-muted-foreground`
- 맨 아래 고정 항목(휴지통 등): `mt-auto border-t border-sidebar-border pt-2`

### 목록 보기 (`FileList`)

- `<table className="table-fixed border-collapse text-sm">`, 첫 열은 남는 폭, 나머지 열은 px(사용자가 끌어 조절, localStorage 기억)
- 머리글: `sticky top-0 h-7 border-b text-2xs text-muted-foreground`, 셀 hover `bg-accent`, 정렬 중인 열만 화살표 `size-3`
- 행 `h-8.5`(34px), 상태별 클래스:

| 상태 | 클래스 |
| --- | --- |
| 줄무늬(홀수 행) | `bg-muted/60` |
| hover | `bg-accent` |
| 선택 | `bg-selection` (줄무늬 대신) |
| 키보드 포커스 | `outline-1 -outline-offset-1 outline-primary/60` |
| 끌어 놓을 대상 | `bg-primary/10 outline-2 -outline-offset-2 outline-primary` |

- 보조 열(수정일·크기): `text-muted-foreground`, 숫자는 `tabular-nums text-right`
- 정렬: 폴더 먼저(이름순), 파일은 정렬 기준대로 — `orderEntries()`, `Intl.Collator` 하나를 재사용
- 로딩: 같은 높이 줄무늬 행 8개에 `animate-pulse` 막대

### 아이콘 보기 (`IconGrid`)

- 셀: `flex flex-col items-center gap-1 rounded-md p-2`, hover `bg-accent/60`, 선택·포커스·드롭 대상은 목록 보기와 같은 클래스
- 라벨: `line-clamp-2 text-center text-xs leading-tight break-all`
- 크기 4단계 (`ICON_SIZES`): 셀 폭 84 / 104 / 140 / 196px. 썸네일은 `rounded-sm object-contain shadow-item`, 폴더는 `FolderGlyph`

### 세부 정보 창 · 상태 막대 · 빈 상태

- 세부 정보: `w-70 border-l bg-sidebar p-4 gap-4`, 미리보기 칸 `h-40 rounded-lg bg-background`, 속성은 `<dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 text-xs">` (`dt`는 `text-muted-foreground`)
- 상태 막대: `h-6 border-t bg-sidebar px-3 text-2xs text-muted-foreground`, 용량 막대는 `Progress` + 트랙 `h-1.5 bg-foreground/15`
- 빈 상태: `flex h-full items-center justify-center text-sm text-muted-foreground` 문장 한 줄 (그림 없음)
- 오류 배너: `border-b border-destructive/30 bg-destructive/10 px-3 py-1.5 text-sm text-destructive` + 오른쪽 "다시 시도"

### 상호작용 (탐색기 관례)

| 동작 | 구현 |
| --- | --- |
| 클릭 / Ctrl·⌘ / Shift 선택 | `MainScreen`의 `onSelect` |
| 빈 곳 끌어 사각형 선택 | `useMarquee` — 사각형 `border border-primary/60 bg-primary/10` |
| 아이콘·이름을 끌어 폴더로 옮기기 | `useDragMove` — `data-drag` 붙은 요소만 손잡이, 따라다니는 라벨 `bg-popover shadow-dialog text-xs` |
| 이름 바꾸기·새 폴더 | `InlineName` — Enter 확정, Esc 취소, 포커스 잃으면 확정, 잘못된 이름은 빨간 테두리 |
| 우클릭 | 항목은 `EntryContextMenu`, 빈 곳은 `BackgroundContextMenu` (우클릭한 항목이 선택 안 됐으면 먼저 선택) |
| 큰 폴더 | `useWindow` — 높이가 같은 줄만 화면 근처로 그림 (2만 개도 수십 줄) |

### 재사용할 만한 순수 훅

UI·저장소와 무관해서 그대로 복사해도 되는 것들:

- `src/lib/useStoredState.ts` — `useState` + localStorage(JSON), 실패하면 메모리만
- `src/features/browse/useWindow.ts` — 고정 높이 가상 스크롤 (`visibleRange` 순수 함수 + 테스트)
- `src/features/browse/useMarquee.ts` — 사각형 선택 (`rectFrom`, `intersects` 순수 함수 + 테스트)
- `src/features/browse/useColumnWidths.ts`, `useSidebarWidth.ts` — 경계 끌어 너비 조절 + 기억

## 5. 다른 프로젝트로 이식하기

### 1) 의존성

```bash
npm i tailwindcss @tailwindcss/vite tw-animate-css shadcn \
      @base-ui/react class-variance-authority cn \
      lucide-react @tabler/icons-react pretendard
# 마크다운 미리보기(prose)를 쓸 때만
npm i @tailwindcss/typography
```

### 2) 복사할 파일

| 원본 | 대상 | 비고 |
| --- | --- | --- |
| `src/index.css` | 앱 전역 CSS | 맨 아래 `.prose-markdown`, `bg-checker`는 필요 없으면 삭제 |
| `src/components/ui/*` | 같은 경로 | `desktop-style.test.ts`는 Vitest가 있으면 같이 가져간다 |
| `src/lib/utils.ts` | 같은 경로 | `export { cn } from "cn"` 한 줄 |
| `components.json` | 프로젝트 루트 | 이후 `npx shadcn add`가 `base-nova`로 받게 |

FSD 구조 프로젝트라면 `components/ui` → `shared/ui`, `lib/utils` → `shared/lib/utils`로 옮기고 `components.json`의 `aliases`와 컴포넌트 안 import(`@/components/ui/button`)를 같이 고친다.

### 3) 설정

- `vite.config.ts`: `plugins: [react(), tailwindcss()]`, `resolve.alias: { "@": path.resolve(__dirname, "./src") }`
- `tsconfig.json`: `"paths": { "@/*": ["./src/*"] }`
- 진입점에서 `import "./index.css"`, 맥 보정이 필요하면 `<html data-os="mac">` 설정
- Next.js라면 `@tailwindcss/vite` 대신 `@tailwindcss/postcss`를 쓰고, Base UI 컴포넌트 파일 맨 위에 `"use client"`를 붙인다

### 4) 새 컴포넌트가 필요할 때

1. `npx shadcn@latest add <name>` — `base-nova`라 Base UI 버전이 들어온다
2. 받은 파일에서 `zoom-*`, `slide-in-from-*`, `translate-y-px`를 지우고 페이드만 남긴다
3. 메뉴류면 클래스를 `menu-styles.ts`의 것으로 바꾼다
4. shadcn에 없으면 `components/ui/<name>.tsx`에 토큰 클래스만으로 직접 작성한다

### 5) 지키는 장치

- `scripts/lint.mjs` — 화면 코드(`src/features`, `App.tsx`)의 `#hex`, `bg-[…]`/`text-[…]`/`border-[…]`, `gray|slate|zinc|neutral|stone|blue|red|green-숫자`를 잡는다. 두 번째 검사 블록만 떼어 가져가면 된다 (첫 번째는 U-Drive 저장소 전용).
- `desktop-style.test.ts` — 데스크톱 토큰이 라이트·다크 둘 다 있는지, 메뉴가 `menu-styles`를 쓰는지, 확대·미끄러짐 애니메이션이 없는지, 버튼 높이·테두리를 검사한다.

### 체크리스트

- [ ] 의존성 설치, `index.css` · `components/ui` · `lib/utils.ts` · `components.json` 복사
- [ ] `@` 별칭(vite + tsconfig) 설정, 경로를 바꿨다면 import 수정
- [ ] `<Button>`, `<Dialog>`, 우클릭 메뉴 하나씩 띄워 라이트 / `class="dark"` 둘 다 확인
- [ ] 브랜드가 다르면 `--primary` 계열(`-hover`, `-active`, `-edge`, `-edge-bottom`), `--brand-*`, `--folder-*`, `--selection`을 같이 바꾼다
- [ ] lint 스크립트와 `desktop-style.test.ts`를 CI에 넣는다
