/** Tauri 앱 안에서 실행 중인지 (브라우저 미리보기·테스트에서는 false) */
export const isTauri = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
