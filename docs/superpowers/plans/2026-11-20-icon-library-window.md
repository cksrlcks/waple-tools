# 아이콘 패널 별도 창 (설계 12장) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development.

**Spec:** [../specs/2026-11-20-icon-library-design.md](../specs/2026-11-20-icon-library-design.md) §12

**Global Constraints:** 2판 계획의 제약 그대로 (앱 코드는 공용, 토큰만, 한국어, 한 파일 한 컴포넌트, 네트워크 없음, 기존 도구 무변경). 새 창 권한은 최소.

### Task 1: 창 열기·닫기 (Rust) + 패널 창 화면 + 메인 연동 + DLL 플래그
- Rust(app/src-tauri): 명령 `open_panel_window(title: String)`(없으면 만들고 있으면 앞으로; 위치 계산은 순수 함수 `panel_position(main_pos, main_size, monitor_pos, monitor_size, panel_size) -> (x, y)` + 테스트), `close_panel_window()`; 창이 닫힐 때 메인에 `panel:closed` 이벤트. capability `capabilities/panel.json`(windows ["panel"], 최소 권한).
- 프론트: 진입점에서 창 라벨이 `panel`이면 패널 앱을 그림(스플래시 없음, 테마 동기화). 패널 앱은 `panel:state`를 받아 §11 시트 내용을 그리고, 옵션 변경은 `panel:set`, X는 `panel:close`. 머리 줄 `data-tauri-drag-region`.
- 메인 작업대: `panelPosition === "window"`면 아래 시트 대신 창을 열고/닫고 상태를 보냄(`panel:ready` 응답 포함), `panel:set`을 applyOption으로 반영, `panel:closed`/`panel:close`면 `x-panel-when` 옵션을 비움, 언마운트 때 닫기. 순수 부분(상태 페이로드 만들기 등)은 lib + 테스트.
- DLL: `x-panel-position: "window"` + 테스트.
- 검사: fmt·clippy·`cargo test --workspace`·`npm test`·`tsc`·`npm run build`.
