export const iconSetKeys = {
  all: ["iconSets"] as const,
  installed: () => [...iconSetKeys.all, "installed"] as const,
  set: (prefix: string) => [...iconSetKeys.all, "set", prefix] as const,
  collections: () => [...iconSetKeys.all, "collections"] as const,
}
