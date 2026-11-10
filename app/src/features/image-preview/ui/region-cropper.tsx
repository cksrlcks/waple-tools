import { useEffect } from "react"
import { useImageCropper } from "@ark-ui/react"

import { cn } from "@/shared/lib/utils"
import {
  ImageCropperImage,
  ImageCropperRootProvider,
  ImageCropperSelection,
  ImageCropperViewport,
} from "@/shared/ui/image-cropper"

import { scaleCrop, type Rect, type Size } from "../lib/scale-crop"

type Props = {
  /** 회전·뒤집기를 이미 적용한 화면용 이미지 */
  src: string
  /** 같은 방향의 원본 크기 (돌려줄 영역의 기준) */
  target: Size
  /** 가로/세로 비율 고정, null이면 자유 */
  ratio: number | null
  onChange: (rect: Rect) => void
  className?: string
}

/** Ark UI 크로퍼. 영역·확대·이동이 바뀔 때마다 원본 기준 px 영역을 알린다 */
export function RegionCropper({
  src,
  target,
  ratio,
  onChange,
  className,
}: Props) {
  const cropper = useImageCropper({
    aspectRatio: ratio ?? undefined,
    minWidth: 8,
    minHeight: 8,
  })
  const n = cropper.naturalSize
  const rect =
    n.width > 0
      ? scaleCrop(cropper.getCropData(), { w: n.width, h: n.height }, target)
      : null
  const key = rect ? `${rect.x},${rect.y},${rect.w},${rect.h}` : ""
  useEffect(() => {
    if (rect) onChange(rect)
    // 영역 값이 바뀔 때만 알린다 (부모가 다시 그려도 같은 값이면 무시)
  }, [key])

  return (
    // 사용 가능한 칸 안에 이미지 비율 그대로 최대 크기로 (container 단위)
    <div
      className={cn("flex size-full items-center justify-center", className)}
      style={{ containerType: "size" }}
    >
      <ImageCropperRootProvider value={cropper}>
        <ImageCropperViewport
          style={{
            aspectRatio: `${target.w} / ${target.h}`,
            width: `min(100cqw, calc(100cqh * ${target.w / target.h}))`,
          }}
        >
          <ImageCropperImage src={src} />
          <ImageCropperSelection />
        </ImageCropperViewport>
      </ImageCropperRootProvider>
      {rect && (
        <span className="bg-stage/80 text-stage-foreground pointer-events-none absolute top-0 left-0 rounded-md px-2 py-1 text-xs tabular-nums">
          {rect.w}×{rect.h}
        </span>
      )}
    </div>
  )
}
