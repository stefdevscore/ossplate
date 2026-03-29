import {
  assertPublishReadiness,
  readRootPackage
} from "./release-state.mjs";

const mode = process.argv[2] ?? "publish";
const explicitVersion = process.argv[3];
const rootPackage = readRootPackage();

const version = explicitVersion ?? rootPackage.version;

main();

function main() {
  assertPublishReadiness(mode, version, rootPackage);
  console.log(`publish readiness ok (${mode}, ${version})`);
}
