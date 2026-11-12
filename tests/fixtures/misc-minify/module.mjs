// ES 모듈: import·export와 최상위 await
import { readFile } from "node:fs/promises";

export async function loadConfiguration(configurationPath) {
  const configurationText = await readFile(configurationPath, "utf8");
  const parsedConfiguration = JSON.parse(configurationText);
  return parsedConfiguration;
}

export const defaultConfiguration = await loadConfiguration("./config.json");
