/** 경로의 마지막 이름 (`\`·`/` 둘 다) */
export const baseName = (p: string) => p.split(/[\\/]/).pop() || p
