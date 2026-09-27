// `npm run serve:e2e`: the end-to-end service, seeded, until Ctrl-C.
// For looking at the UI by hand as the tests see it.

import { ADMIN, BASE_URL, PASSWORD, start } from "./harness.ts";

const running = await start();
process.stdout.write(`${BASE_URL}  (${ADMIN} / ${PASSWORD})\n`);
const stop = () => {
  void running.stop().then(() => process.exit(0));
};
process.on("SIGINT", stop);
process.on("SIGTERM", stop);
