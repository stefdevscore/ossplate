import {
  assertPublishReadiness,
  assertReleaseState,
  assertGeneratedScaffoldAssets,
  assertNoRepoPackagingLeakage,
  readRootPackage,
  readScaffoldPayload
} from "./release-state.mjs";

const [command, ...rest] = process.argv.slice(2);

main();

function main() {
  switch (command) {
    case "release-state":
      assertReleaseState(readRootPackage());
      console.log("release state ok");
      return;
    case "scaffold-mirrors":
    case "scaffold-assets":
      assertGeneratedScaffoldAssets(readScaffoldPayload());
      console.log("scaffold assets ok");
      return;
    case "package-cleanliness":
      assertNoRepoPackagingLeakage();
      console.log("package cleanliness ok");
      return;
    case "publish-readiness": {
      const mode = rest[0] ?? "publish";
      const rootPackage = readRootPackage();
      const version = rest[1] ?? rootPackage.version;
      assertPublishReadiness(mode, version, rootPackage);
      console.log(`publish readiness ok (${mode}, ${version})`);
      return;
    }
    default:
      throw new Error(
        "usage: node scripts/release-check.mjs <release-state|scaffold-mirrors|scaffold-assets|package-cleanliness|publish-readiness> [args]"
      );
  }
}
