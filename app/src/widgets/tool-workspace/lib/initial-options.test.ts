import { describe, expect, it } from "vitest"

import type { Field } from "@/features/tool-options"

import { initialOptions } from "./initial-options"

const metadata: Field = {
  kind: "select",
  key: "metadata",
  label: "메타데이터",
  options: [
    { value: "strip", label: "지우기" },
    { value: "keep", label: "유지" },
  ],
  default: "strip",
}
const quality: Field = {
  kind: "slider",
  key: "quality",
  label: "품질",
  min: 0,
  max: 100,
  step: 1,
  default: 80,
}

describe("initialOptions", () => {
  it("저장된 값이 없으면 메타데이터는 설정값을 따른다", () => {
    expect(initialOptions([quality, metadata], undefined, true)).toEqual({
      quality: 80,
      metadata: "keep",
    })
    expect(initialOptions([quality, metadata], undefined, false)).toEqual({
      quality: 80,
      metadata: "strip",
    })
  })
  it("저장된 메타데이터 값이 있으면 그것을 쓴다", () => {
    expect(initialOptions([metadata], { metadata: "strip" }, true)).toEqual({
      metadata: "strip",
    })
  })
  it("메타데이터 옵션이 없는 도구는 그대로", () => {
    expect(initialOptions([quality], undefined, true)).toEqual({ quality: 80 })
  })
})
