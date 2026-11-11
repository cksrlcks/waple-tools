export const interactiveKeys = {
  all: ["interactive"] as const,
  run: (toolId: string, path: string | null, opts: string) =>
    [...interactiveKeys.all, toolId, path, opts] as const,
}
