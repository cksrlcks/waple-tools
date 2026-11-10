/** 릴리스 노트(CHANGELOG 섹션)를 알림용 평문으로. 마크다운 기호·빈 줄은 빼고 목록은 •로 */
export const noteText = (body?: string) =>
  body
    ?.split("\n")
    .map((l) => l.replace(/[#*`]/g, "").trim().replace(/^- /, "• "))
    .filter(Boolean)
    .join("\n")
