/** 도구가 받은 리소스 하나. `latest`는 업데이트 확인으로 알게 된 새 버전 (없으면 null) */
export type ResourceState = {
  id: string
  version: string
  latest: string | null
}
