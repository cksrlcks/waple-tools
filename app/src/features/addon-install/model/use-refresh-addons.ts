import { useEffect } from "react"
import { useQueryClient } from "@tanstack/react-query"

import { toolKeys } from "@/entities/tool"

import { refreshAddons } from "../api/addon-api"

/** 앱을 켤 때 한 번 원격 도구 목록을 조용히 받고, 업데이트 표시를 갱신한다 (실패해도 넘어간다) */
export function useRefreshAddons() {
  const qc = useQueryClient()
  useEffect(() => {
    refreshAddons()
      .then(() => qc.invalidateQueries({ queryKey: toolKeys.all }))
      .catch(() => {})
  }, [qc])
}
