import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

exec("npm", ["install", "--package-lock-only"], join(repoRoot, "wrapper-js"));
exec("node", [join(repoRoot, "scripts", "assert-js-lockfile-state.mjs"), "resolved"], repoRoot);
exec(
  "cargo",
  ["run", "--quiet", "--manifest-path", join(repoRoot, "core-rs", "Cargo.toml"), "--", "sync", "--path", repoRoot],
  repoRoot
);
exec(
  "node",
  [join(repoRoot, "scripts", "stage-distribution-assets.mjs"), "embedded-template"],
  repoRoot
);

function exec(command, args, cwd) {
  execFileSync(command, args, {
    cwd,
    stdio: "inherit"
  });
}
