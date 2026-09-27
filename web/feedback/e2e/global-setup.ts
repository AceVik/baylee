import { start } from "./harness.ts";

export default async function globalSetup(): Promise<() => Promise<void>> {
  const running = await start();
  return running.stop;
}
