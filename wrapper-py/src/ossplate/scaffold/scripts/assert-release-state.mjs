import { assertReleaseState, readRootPackage } from "./release-state.mjs";

main();

function main() {
  assertReleaseState(readRootPackage());
  console.log("release state ok");
}
