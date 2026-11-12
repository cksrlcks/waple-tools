import { describe, expect, it } from "vitest"

import { rememberedOptions } from "./remembered-options"
import { schemaToFields } from "./schema-to-fields"

describe("rememberedOptions", () => {
  it("비밀번호 필드는 빼고 나머지는 그대로 둔다", () => {
    const fields = schemaToFields({
      properties: {
        password: { type: "string", format: "password" },
        name: { type: "string" },
      },
    })
    expect(fields[0]).toMatchObject({ kind: "password", default: "" })
    const opts = { password: "secret", name: "a" }
    expect(rememberedOptions(fields, opts)).toEqual({ name: "a" })
    expect(opts.password).toBe("secret")
  })
})
