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
      nullable: true,
      default: null,
    })
  })
  it("null을 허용하지 않는 숫자는 nullable false", () => {
    const f = schemaToFields({
      properties: { n: { type: "integer", minimum: 1, default: 5 } },
    })
    expect(f[0]).toMatchObject({ kind: "number", nullable: false, default: 5 })
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
  it("x-wide 문자열은 넓은 text, 아니면 wide 없음", () => {
    const f = schemaToFields({
      properties: {
        url: { type: "string", default: "", "x-wide": true },
        name: { type: "string", default: "" },
      },
    })
    expect(f[0]).toMatchObject({ kind: "text", wide: true })
    expect(f[1]).not.toHaveProperty("wide")
  })
  it("format file → file, 확장자는 x-file-extensions (없으면 빈 목록)", () => {
    const f = schemaToFields({
      properties: {
        logo: {
          type: "string",
          format: "file",
          default: "",
          "x-file-extensions": ["png", "webp", 3],
        },
        any: { type: "string", format: "file" },
      },
    })
    expect(f[0]).toMatchObject({
      kind: "file",
      default: "",
      extensions: ["png", "webp"],
    })
    expect(f[1]).toMatchObject({ kind: "file", default: "", extensions: [] })
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
  it("옵션 연결·묶음 확장 키를 읽는다", () => {
    const f = schemaToFields({
      properties: {
        compress: {
          type: "boolean",
          default: true,
          "x-group": "용량",
          "x-group-toggle": true,
        },
        quality: {
          type: "integer",
          minimum: 1,
          maximum: 100,
          default: 80,
          "x-group": "용량",
          "x-enabled-when": { compress: [true], mode: ["quality"] },
        },
        background: {
          type: "string",
          format: "color",
          default: "#ffffff",
          "x-enabled-when": [{ format: ["jpg"] }, { fit: ["pad"] }],
        },
      },
    })
    expect(f[0]).toMatchObject({
      kind: "checkbox",
      group: "용량",
      groupToggle: true,
    })
    expect(f[1]).toMatchObject({
      group: "용량",
      enabledWhen: [{ compress: [true], mode: ["quality"] }],
    })
    expect(f[2]).toMatchObject({
      enabledWhen: [{ format: ["jpg"] }, { fit: ["pad"] }],
    })
  })
  it("잘못된 스키마 → 빈 목록", () => {
    expect(schemaToFields(null)).toEqual([])
    expect(schemaToFields("x")).toEqual([])
    expect(schemaToFields({})).toEqual([])
  })
})

describe("라벨과 물음표 설명", () => {
  it("description의 첫 문단은 라벨, 빈 줄 뒤는 설명", () => {
    const [f] = schemaToFields({
      properties: {
        quality: {
          type: "integer",
          description: "품질\n\nPNG는 화질 손실 없이 압축만 다시 합니다.",
        },
      },
    })
    expect(f?.label).toBe("품질")
    expect(f && "description" in f && f.description).toBe(
      "PNG는 화질 손실 없이 압축만 다시 합니다.",
    )
  })
  it("빈 줄이 없으면 전체가 라벨, 설명 없음", () => {
    const [f] = schemaToFields({
      properties: { q: { type: "integer", description: "품질" } },
    })
    expect(f?.label).toBe("품질")
    expect(f && "description" in f ? f.description : undefined).toBeUndefined()
  })
})
