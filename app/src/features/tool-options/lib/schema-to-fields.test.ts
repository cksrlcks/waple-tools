import { describe, expect, it } from "vitest"

import { schemaToFields } from "./schema-to-fields"

// schemars 1이 실제로 내는 형태
const schema = {
  $schema: "https://json-schema.org/draft/2020-12/schema",
  title: "WebpOptions",
  type: "object",
  properties: {
    quality: {
      type: "integer",
      format: "uint8",
      minimum: 0,
      maximum: 100,
      default: 80,
      description: "품질",
    },
    lossless: { type: "boolean", default: false, description: "무손실" },
    mode: { $ref: "#/$defs/Mode", default: "quality", description: "모드" },
    sizes: {
      type: "array",
      items: { type: "integer" },
      description: "너비 목록",
    },
    targetKb: {
      type: ["integer", "null"],
      minimum: 1,
      default: null,
      description: "목표 KB",
    },
    background: {
      type: "string",
      format: "color",
      default: "#ffffff",
      description: "배경색",
    },
    suffix: {
      type: "string",
      default: "",
      title: "접미사",
      description: "파일 이름 뒤에 붙입니다",
    },
  },
  $defs: {
    Mode: {
      oneOf: [
        { type: "string", const: "quality", description: "품질 지정" },
        { type: "string", const: "size", description: "목표 KB" },
      ],
    },
  },
}

describe("schemaToFields", () => {
  const fields = schemaToFields(schema)
  const byKey = Object.fromEntries(fields.map((f) => [f.key, f]))

  it("properties 순서를 지킨다", () => {
    expect(fields.map((f) => f.key)).toEqual([
      "quality",
      "lossless",
      "mode",
      "sizes",
      "targetKb",
      "background",
      "suffix",
    ])
  })
  it("범위가 둘 다 있는 정수 → slider", () => {
    expect(byKey.quality).toEqual({
      kind: "slider",
      key: "quality",
      label: "품질",
      min: 0,
      max: 100,
      step: 1,
      default: 80,
    })
  })
  it("boolean → checkbox", () => {
    expect(byKey.lossless).toMatchObject({
      kind: "checkbox",
      label: "무손실",
      default: false,
    })
  })
  it("$ref + oneOf const → select, 라벨은 description", () => {
    expect(byKey.mode).toMatchObject({
      kind: "select",
      default: "quality",
      options: [
        { value: "quality", label: "품질 지정" },
        { value: "size", label: "목표 KB" },
      ],
    })
  })
  it("배열 → unsupported", () => {
    expect(byKey.sizes).toEqual({
      kind: "unsupported",
      key: "sizes",
      label: "너비 목록",
    })
  })
  it("범위가 하나뿐인 nullable 정수 → number", () => {
    expect(byKey.targetKb).toMatchObject({
      kind: "number",
      min: 1,
      integer: true,
      default: null,
    })
  })
  it("format color → color", () => {
    expect(byKey.background).toMatchObject({
      kind: "color",
      default: "#ffffff",
    })
  })
  it("title이 있으면 description은 설명으로", () => {
    expect(byKey.suffix).toMatchObject({
      kind: "text",
      label: "접미사",
      description: "파일 이름 뒤에 붙입니다",
    })
  })
  it("enum 문자열도 select", () => {
    const f = schemaToFields({
      properties: {
        fmt: { type: "string", enum: ["jpg", "png"], default: "png" },
      },
    })
    expect(f[0]).toMatchObject({
      kind: "select",
      options: [
        { value: "jpg", label: "jpg" },
        { value: "png", label: "png" },
      ],
      default: "png",
    })
  })
  it("잘못된 스키마 → 빈 목록", () => {
    expect(schemaToFields(null)).toEqual([])
    expect(schemaToFields("x")).toEqual([])
    expect(schemaToFields({})).toEqual([])
  })
})
