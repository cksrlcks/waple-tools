import { useEffect, useState } from "react"

import type { Shown } from "@/entities/tool"

import { transformImage } from "../lib/transform-image"
import type { RegionEdit } from "../model/region-edit"
import { PreviewNotice } from "./preview-notice"
import { RegionCropper } from "./region-cropper"

type Props = {
  /** 원본 (url은 화면용으로 줄어 있을 수 있고, width·height는 원본 px) */
  shown: Shown
  edit: RegionEdit
  className?: string
}

/** 회전·뒤집기를 반영한 원본 위에서 자를 영역을 끌어 정한다 */
export function RegionEditor({ shown, edit, className }: Props) {
  const { angle, flipH, flipV } = edit
  const [src, setSrc] = useState<string | null>(null)
  useEffect(() => {
    let alive = true
    transformImage(shown.url, angle, flipH, flipV)
      .then((u) => alive && setSrc(u))
      .catch(() => alive && setSrc(shown.url))
    return () => {
      alive = false
    }
  }, [shown.url, angle, flipH, flipV])

  if (!src) return <PreviewNotice>이미지를 준비하는 중…</PreviewNotice>
  const turned = angle % 180 !== 0
  const target = turned
    ? { w: shown.height, h: shown.width }
    : { w: shown.width, h: shown.height }
  return (
    // 이미지·비율이 바뀌면 크로퍼를 새로 만들어 기본 영역부터 다시
    <RegionCropper
      key={`${edit.ratio}|${src}`}
      className={className}
      src={src}
      target={target}
      ratio={edit.ratio}
      onChange={edit.onChange}
    />
  )
}
