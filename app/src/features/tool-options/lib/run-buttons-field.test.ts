import { describe, expect, it } from "vitest"

import { runButtonsField } from "./run-buttons-field"

const schema = {
  type: "object",
  properties: {
    other: { type: "boolean", default: false, description: "다른 옵션" },
    mode: {
      $ref: "#/$defs/M",
      default: "deutan",
      description: "보기 방식",
      "x-run-buttons": true,
    },
  },
  $defs: {
    M: {
      oneOf: [
        { type: "string", const: "protan", description: "적색맹" },
        { type: "string", const: "deutan", description: "녹색맹" },
      ],
    },
  },
}

describe("runButtonsField", () => {
  it("x-run-buttons 필드를 찾는다", () => {
    expect(runButtonsField(schema)?.key).toBe("mode")
  })
  it("없으면 null", () => {
    expect(runButtonsField({ type: "object", properties: {} })).toBeNull()
  })
})
