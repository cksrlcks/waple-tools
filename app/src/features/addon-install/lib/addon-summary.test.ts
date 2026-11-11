import { describe, expect, it } from "vitest"

import type { ToolEntry } from "@/entities/tool"

import { addonSummary, isInstalledAddon } from "./addon-summary"
import { downloadPercent } from "./download-percent"

const tool = (patch: Partial<ToolEntry>): ToolEntry => ({
  id: "image.base64",
  name: "이미지 → Base64",
  category: "image",
  description: "",
  status: "installed",
  kind: "interactive",
  optionsSchema: {},
  extensions: [],
  version: "1.0.0",
  latest: "1.0.0",
  size: 100,
  ...patch,
})

describe("추가 도구 요약", () => {
  it("설치 기록이 있는 상태만 목록에 넣는다", () => {
    expect(isInstalledAddon("installed")).toBe(true)
    expect(isInstalledAddon("updateRequired")).toBe(true)
    expect(isInstalledAddon("notInstalled")).toBe(false)
    expect(isInstalledAddon("builtin")).toBe(false)
  })

  it("버전과 새 버전을 보여 준다", () => {
    expect(addonSummary(tool({}))).toBe("버전 1.0.0")
    expect(
      addonSummary(tool({ status: "updateAvailable", latest: "1.1.0" })),
    ).toBe("버전 1.0.0 · 새 버전 1.1.0")
    expect(addonSummary(tool({ status: "updateRequired" }))).toBe(
      "다시 설치해야 쓸 수 있습니다.",
    )
  })

  it("받은 비율", () => {
    expect(downloadPercent(null)).toBeNull()
    expect(downloadPercent({ received: 5, total: null })).toBeNull()
    expect(downloadPercent({ received: 1, total: 3 })).toBe(33)
    expect(downloadPercent({ received: 9, total: 3 })).toBe(100)
  })
})
