import { assertScaffoldMirrorsState, readScaffoldPayload } from "./release-state.mjs";

main();

function main() {
  assertScaffoldMirrorsState(readScaffoldPayload());
  console.log("scaffold mirrors ok");
}
