import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { updateFormula } from "./update-homebrew-formula.mjs";

const platforms = [
  "darwin-arm64",
  "darwin-x64",
  "linux-arm64-gnu",
  "linux-x64-gnu",
];

const oldFormula = `class Hashjunkie < Formula
  desc "Multi-algorithm file hasher"
  homepage "https://github.com/Tuxie/HashJunkie"
  license "MIT"

  release_version = "0.3.0"

  on_macos do
    on_arm do
      url "https://github.com/Tuxie/HashJunkie/releases/download/v#{release_version}/hashjunkie-cli-#{release_version}-darwin-arm64.tar.xz"
      sha256 "old-darwin-arm64"
    end
    on_intel do
      url "https://github.com/Tuxie/HashJunkie/releases/download/v#{release_version}/hashjunkie-cli-#{release_version}-darwin-x64.tar.xz"
      sha256 "old-darwin-x64"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/Tuxie/HashJunkie/releases/download/v#{release_version}/hashjunkie-cli-#{release_version}-linux-arm64-gnu.tar.xz"
      sha256 "old-linux-arm64"
    end
    on_intel do
      url "https://github.com/Tuxie/HashJunkie/releases/download/v#{release_version}/hashjunkie-cli-#{release_version}-linux-x64-gnu.tar.xz"
      sha256 "old-linux-x64"
    end
  end

  def install
    bin.install "hashjunkie"
  end

  test do
    output = pipe_output("#{bin}/hashjunkie -a sha256 --format hex", "abc")
    assert_match "sha256: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", output
  end
end
`;

test("updateFormula writes release version and archive SHA256s", async () => {
  const root = await mkdtemp(join(tmpdir(), "hashjunkie-formula-"));
  try {
    const formula = join(root, "hashjunkie.rb");
    const artifacts = join(root, "artifacts");
    await writeFile(formula, oldFormula);
    await mkdir(artifacts);

    const expected = {};
    for (const platform of platforms) {
      const contents = Buffer.from(`archive:${platform}`);
      expected[platform] = createHash("sha256").update(contents).digest("hex");
      await writeFile(
        join(artifacts, `hashjunkie-cli-0.3.1-${platform}.tar.xz`),
        contents,
      );
    }

    await updateFormula({ formula, artifacts, version: "0.3.1" });

    const updated = await readFile(formula, "utf8");
    assert.match(updated, /release_version = "0\.3\.1"/);
    for (const [platform, sha] of Object.entries(expected)) {
      assert(updated.includes(`hashjunkie-cli-#{release_version}-${platform}.tar.xz`));
      assert(updated.includes(`sha256 "${sha}"`));
    }
    assert(!updated.includes("old-darwin-arm64"));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
