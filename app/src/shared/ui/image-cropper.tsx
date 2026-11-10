import type { ComponentProps } from "react"
import { ImageCropper as Cropper } from "@ark-ui/react"

import { cn } from "@/shared/lib/utils"

/** Ark UI ImageCropper를 앱 토큰으로 꾸민 부품 (shadcn image-cropper와 같은 구성) */

const HANDLES = ["nw", "n", "ne", "e", "se", "s", "sw", "w"] as const

function ImageCropperRootProvider(
  props: ComponentProps<typeof Cropper.RootProvider>,
) {
  return <Cropper.RootProvider data-slot="image-cropper" {...props} />
}

function ImageCropperViewport({
  className,
  ...props
}: ComponentProps<typeof Cropper.Viewport>) {
  return (
    <Cropper.Viewport
      data-slot="image-cropper-viewport"
      className={cn("rounded-md", className)}
      {...props}
    />
  )
}

function ImageCropperImage({
  className,
  ...props
}: ComponentProps<typeof Cropper.Image>) {
  return (
    <Cropper.Image
      data-slot="image-cropper-image"
      className={cn("block size-full", className)}
      {...props}
    />
  )
}

/** 자를 영역: 바깥은 어둡게, 3등분 안내선과 손잡이 8개 */
function ImageCropperSelection({
  className,
  ...props
}: ComponentProps<typeof Cropper.Selection>) {
  return (
    <Cropper.Selection
      data-slot="image-cropper-selection"
      className={cn(
        "border-stage-foreground focus-visible:ring-primary cursor-move border shadow-[0_0_0_9999px_color-mix(in_oklch,var(--stage)_60%,transparent)] outline-none focus-visible:ring-2",
        className,
      )}
      {...props}
    >
      {/* 위치는 Ark가 잡는다(가운데 1/3 상자): 테두리가 3등분 선이 된다 */}
      <Cropper.Grid axis="horizontal" className="border-stage-foreground/40 border-y" />
      <Cropper.Grid axis="vertical" className="border-stage-foreground/40 border-x" />
      {HANDLES.map((position) => (
        // 손잡이 자리는 Ark가 잡는 투명 클릭 영역, 보이는 건 가운데 작은 점
        <Cropper.Handle
          key={position}
          position={position}
          className="flex items-center justify-center"
        >
          <span className="bg-stage-foreground border-stage size-2.5 rounded-xs border" />
        </Cropper.Handle>
      ))}
    </Cropper.Selection>
  )
}

export {
  ImageCropperImage,
  ImageCropperRootProvider,
  ImageCropperSelection,
  ImageCropperViewport,
}
