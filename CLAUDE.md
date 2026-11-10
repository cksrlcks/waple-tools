# 작업 규칙

- docs/plan.md와 docs/superpowers/specs/2026-11-10-image-first-addon-design.md가 기준 문서다. 지시받은 Phase 범위 밖의 기능은 만들지 않는다.
- UI 작업은 docs/UI-SYSTEM.md를 따른다. 화면 코드에 #hex, bg-[…], text-gray-500 같은 토큰 밖 색을 쓰지 않는다.
- AI/머신러닝 모델(OCR, 배경 제거, 업스케일, 이미지 생성 등)을 쓰는 기능은 추가하지 않는다.
- 네트워크는 app/src-tauri의 추가 도구 설치 모듈에서만 쓴다. 그 밖의 곳에 네트워크 크레이트, Tauri http 권한, 외부 CDN 리소스를 추가하지 않는다.
- 원본 파일을 삭제하거나 덮어쓰는 코드는 금지한다. 도구는 out_dir에만 쓰고, 파일 이동은 반드시 core의 배치 엔진을 거친다.
- 작업 시작 전 변경 계획을 짧게 제시하고, 끝나면 아래를 모두 통과시킨다.
  - cargo fmt --check
  - cargo clippy --workspace -- -D warnings
  - cargo test --workspace
- 사용자 파일을 다루는 코드는 임시 폴더 기반 테스트를 함께 작성한다. 테스트용 샘플은 tests/fixtures에 둔다.
- 새 도구는 crates/tools/<카테고리>-<이름>/ 크레이트 하나로 만들고 catalog/tools.json에 추가한다.
  기본 내장이면 app/src-tauri의 builtin()에 등록, 추가 설치면 export_tool! 한 줄로 DLL을 내보낸다.
  도구 전용 UI 코드는 작성하지 않고 옵션 스키마와 결과 블록으로 해결한다. 불가능한 UI가 필요하면 먼저 제안한다.
- 의존성을 추가할 때는 이유와 라이선스를 커밋 메시지에 적는다. GPL/AGPL 라이선스는 사용 전에 확인을 받는다.
- 화면 문구, 옵션 설명, 오류 메시지는 한국어로 작성한다.
- Phase가 끝나면 PLAN.md의 완료 기준을 하나씩 확인한 결과를 보고한다.
