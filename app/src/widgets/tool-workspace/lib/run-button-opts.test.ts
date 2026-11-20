import { describe, expect, it } from "vitest"

import { schemaToFields } from "@/features/tool-options"

import { runButtonOpts } from "./run-button-opts"

const fields = schemaToFields({
  type: "object",
  properties: {
    loud: { type: "boolean", default: true, description: "다른 옵션" },
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
})

describe("runButtonOpts", () => {
  it("기본값에 누른 값을 덮는다", () => {
    expect(runButtonOpts(fields, "mode", "protan")).toEqual({
      loud: true,
      mode: "protan",
    })
  })
})
