import { useCallback, useEffect, useRef, useState } from "react"

import { checkUpdate, restartApp, type Update } from "../api/app-update-api"
import { percentOf, type UpdateStatus } from "../lib/update-label"

/**
 * 앱 업데이트: 켤 때 조용히 한 번 확인하고, 버튼으로 다시 확인·설치한다.
 * "최신입니다."·"확인할 수 없습니다."는 잠시 보였다가 원래 글자로 돌아간다.
 */
export function useAppUpdate() {
  const [status, setStatus] = useState<UpdateStatus>({ kind: "idle" })
  const update = useRef<Update | null>(null)

  const run = useCallback(async (quiet: boolean) => {
    if (!quiet) setStatus({ kind: "checking" })
    try {
      const u = await checkUpdate()
      update.current = u
      if (u) setStatus({ kind: "available", version: u.version })
      else setStatus(quiet ? { kind: "idle" } : { kind: "latest" })
    } catch {
      // 켤 때 확인은 실패해도(오프라인 등) 조용히 넘어간다
      setStatus(quiet ? { kind: "idle" } : { kind: "failed" })
    }
  }, [])

  useEffect(() => {
    void run(true)
  }, [run])

  useEffect(() => {
    if (status.kind !== "latest" && status.kind !== "failed") return
    const t = setTimeout(() => setStatus({ kind: "idle" }), 2000)
    return () => clearTimeout(t)
  }, [status.kind])

  const install = useCallback(async () => {
    const u = update.current
    if (!u) return
    setStatus({ kind: "installing", percent: null })
    let total: number | undefined
    let done = 0
    try {
      await u.downloadAndInstall((e) => {
        if (e.event === "Started") total = e.data.contentLength
        if (e.event === "Progress") {
          done += e.data.chunkLength
          setStatus({ kind: "installing", percent: percentOf(done, total) })
        }
      })
      await restartApp()
    } catch {
      setStatus({ kind: "failed" })
    }
  }, [])

  return { status, check: () => run(false), install }
}
