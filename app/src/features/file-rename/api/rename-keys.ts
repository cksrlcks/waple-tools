export const renameKeys = {
  all: ["rename"] as const,
  plan: (toolId: string, spec: string) =>
    [...renameKeys.all, toolId, spec] as const,
}
