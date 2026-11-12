/** 정규식 치트 시트: 누르면 패턴의 커서 자리에 `insert`를 넣는다 */
export type RegexSnippet = { label: string; insert: string; hint: string }

/** 치트 시트를 접었을 때 한 줄로 보이는 것 */
export const QUICK_SNIPPETS: RegexSnippet[] = [
  { label: "\\d", insert: "\\d", hint: "숫자" },
  { label: "\\w", insert: "\\w", hint: "글자·숫자·_" },
  { label: "\\s", insert: "\\s", hint: "공백" },
  { label: ".", insert: ".", hint: "아무 글자 하나" },
  { label: "[…]", insert: "[]", hint: "이 중 하나" },
  { label: "^", insert: "^", hint: "처음" },
  { label: "$", insert: "$", hint: "끝" },
  { label: "\\b", insert: "\\b", hint: "단어 경계" },
  { label: "+", insert: "+", hint: "1번 이상" },
  { label: "*", insert: "*", hint: "0번 이상" },
  { label: "?", insert: "?", hint: "있거나 없거나" },
  { label: "(…)", insert: "()", hint: "그룹" },
  { label: "(?:…)", insert: "(?:)", hint: "번호 없는 그룹" },
]

export const SNIPPET_GROUPS: { title: string; items: RegexSnippet[] }[] = [
  {
    title: "문자 클래스",
    items: [
      { label: "\\d", insert: "\\d", hint: "숫자" },
      { label: "\\D", insert: "\\D", hint: "숫자가 아닌 것" },
      { label: "\\w", insert: "\\w", hint: "글자·숫자·_" },
      { label: "\\W", insert: "\\W", hint: "\\w가 아닌 것" },
      { label: "\\s", insert: "\\s", hint: "공백" },
      { label: "\\S", insert: "\\S", hint: "공백이 아닌 것" },
      { label: ".", insert: ".", hint: "아무 글자 하나" },
      { label: "[…]", insert: "[]", hint: "이 중 하나" },
      { label: "[a-z]", insert: "[a-z]", hint: "범위" },
      { label: "[^…]", insert: "[^]", hint: "이것 말고" },
    ],
  },
  {
    title: "앵커",
    items: [
      { label: "^", insert: "^", hint: "처음 (여러 줄이면 줄 처음)" },
      { label: "$", insert: "$", hint: "끝 (여러 줄이면 줄 끝)" },
      { label: "\\b", insert: "\\b", hint: "단어 경계" },
      { label: "\\B", insert: "\\B", hint: "단어 경계가 아닌 곳" },
    ],
  },
  {
    title: "수량자",
    items: [
      { label: "+", insert: "+", hint: "1번 이상" },
      { label: "*", insert: "*", hint: "0번 이상" },
      { label: "?", insert: "?", hint: "있거나 없거나" },
      { label: "{n}", insert: "{2}", hint: "정확히 n번" },
      { label: "{n,m}", insert: "{2,4}", hint: "n~m번" },
      { label: "+?", insert: "+?", hint: "가능한 적게" },
    ],
  },
  {
    title: "그룹",
    items: [
      { label: "(…)", insert: "()", hint: "그룹" },
      { label: "(?:…)", insert: "(?:)", hint: "번호 없는 그룹" },
      { label: "(?<name>…)", insert: "(?<name>)", hint: "이름 그룹" },
      { label: "a|b", insert: "|", hint: "또는" },
      { label: "\\1", insert: "\\1", hint: "1번 그룹과 같은 글" },
    ],
  },
  {
    title: "전후방 탐색",
    items: [
      { label: "(?=…)", insert: "(?=)", hint: "뒤에 이것이 있음" },
      { label: "(?!…)", insert: "(?!)", hint: "뒤에 이것이 없음" },
      { label: "(?<=…)", insert: "(?<=)", hint: "앞에 이것이 있음" },
      { label: "(?<!…)", insert: "(?<!)", hint: "앞에 이것이 없음" },
    ],
  },
  {
    title: "자주 쓰는 패턴",
    items: [
      {
        label: "이메일",
        insert: "[\\w.+-]+@[\\w-]+\\.[\\w.-]+",
        hint: "name@example.com",
      },
      { label: "URL", insert: "https?://[^\\s]+", hint: "http(s)://…" },
      {
        label: "IPv4",
        insert: "\\b(?:\\d{1,3}\\.){3}\\d{1,3}\\b",
        hint: "192.168.0.1",
      },
      { label: "날짜", insert: "\\d{4}-\\d{2}-\\d{2}", hint: "2026-11-12" },
      {
        label: "HEX 색",
        insert: "#(?:[0-9a-fA-F]{3}){1,2}\\b",
        hint: "#fff, #1a2b3c",
      },
      {
        label: "휴대폰",
        insert: "01[016789]-?\\d{3,4}-?\\d{4}",
        hint: "010-1234-5678",
      },
    ],
  },
]
