import { toast } from "sonner"

import {
  iconSize,
  toClassSnippet,
  toDataUri,
  toJsx,
  toSvg,
  type IconItem,
  type IconSet,
  type RenderOpts,
} from "@/entities/icon-set"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

import { PNG_SCALES } from "../config/icon-library"
import { saveSvg } from "../lib/save-svg"
import { svgToPngBlob } from "../lib/svg-to-png"

type Props = {
  icon: IconItem
  set: IconSet
  opts: RenderOpts
  className?: string
}

function notify(p: Promise<void>) {
  p.then(() => toast.success("복사했습니다.")).catch(() =>
    toast.error("복사할 수 없습니다."),
  )
}

/** 크기·두께·색을 반영한 SVG를 여러 형식으로 복사·저장 */
export function CopyActions({ icon, set, opts, className }: Props) {
  const svg = toSvg(icon, set, opts)
  const cls = toClassSnippet(set.prefix, icon.name)
  const copyText = (text: string) => notify(navigator.clipboard.writeText(text))

  const copyPng = (scale: number) => {
    const size = opts.size * scale
    const { width: w, height: h } = iconSize(icon, set)
    // canvas에는 글자색이 없으니 currentColor는 검정으로
    const color = opts.color === "currentColor" ? "#000000" : opts.color
    const png = toSvg(icon, set, { ...opts, size, color })
    // Blob 대신 Promise를 넘겨 그리는 동안 클릭 직후의 클립보드 권한을 잃지 않게
    const blob = svgToPngBlob(png, {
      width: Math.round((size * w) / h),
      height: size,
    })
    notify(
      navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]),
    )
  }

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div className="grid grid-cols-2 gap-2">
        <Button variant="outline" onClick={() => copyText(svg)}>
          SVG 복사
        </Button>
        <Button variant="outline" onClick={() => copyText(toJsx(svg))}>
          JSX 복사
        </Button>
        <Button variant="outline" onClick={() => copyText(toDataUri(svg))}>
          Data URI 복사
        </Button>
        {cls && (
          <Button variant="outline" onClick={() => copyText(cls)}>
            클래스 복사
          </Button>
        )}
      </div>
      <div className="grid grid-cols-[1fr_auto] items-center gap-x-3">
        <span className="text-muted-foreground text-sm">PNG 복사</span>
        <div className="flex gap-1">
          {PNG_SCALES.map((s) => (
            <Button key={s} variant="outline" onClick={() => copyPng(s)}>
              {s}x
            </Button>
          ))}
        </div>
      </div>
      <Button onClick={() => void saveSvg(icon.name, svg)}>SVG 저장</Button>
    </div>
  )
}
