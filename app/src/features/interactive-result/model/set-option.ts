/** 블록이 옵션 하나를 바꾼다. `resets`의 옵션은 기본값으로 되돌린다 (`applyOption`) */
export type SetOption = (key: string, value: unknown, resets?: string[]) => void
