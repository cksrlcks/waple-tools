export const toolResourceKeys = {
  all: ["tool-resource"] as const,
  list: (toolId: string) => [...toolResourceKeys.all, toolId] as const,
}
