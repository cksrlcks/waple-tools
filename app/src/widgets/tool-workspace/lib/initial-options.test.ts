import { describe, expect, it } from "vitest"

import { schemaToFields, type Field } from "@/features/tool-options"

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
  it("묶음 스위치는 저장된 값이 켜져 있어도 꺼진 채로 시작", () => {
    const fields = schemaToFields({
      properties: {
        compress: { type: "boolean", default: false, "x-group-toggle": true },
        quality: { type: "integer", default: 100 },
      },
    })
    expect(
      initialOptions(fields, { compress: true, quality: 80 }, false),
    ).toEqual({
      compress: false,
      quality: 80,
    })
  })
  it("여러 줄 글은 저장된 값이 있어도 기본값으로 시작", () => {
    const fields = schemaToFields({
      properties: {
        text: { type: "string", default: "", "x-multiline": true },
      },
    })
    expect(initialOptions(fields, { text: "큰 글" }, false)).toEqual({
      text: "",
    })
  })
})
