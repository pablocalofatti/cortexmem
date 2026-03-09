"use strict";

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const PLATFORM_MAP = {
  "darwin-arm64": "cortexmem-darwin-arm64.tar.gz",
  "darwin-x64": "cortexmem-darwin-x64.tar.gz",
  "linux-arm64": "cortexmem-linux-arm64.tar.gz",
  "linux-x64": "cortexmem-linux-x64.tar.gz",
};

function getArchiveName() {
  const key = `${process.platform}-${process.arch}`;
  const archive = PLATFORM_MAP[key];
  if (!archive) {
    console.error(
      `Unsupported platform: ${process.platform}-${process.arch}. ` +
        `Supported: ${Object.keys(PLATFORM_MAP).join(", ")}`
    );
    process.exit(1);
  }
  return archive;
}

function download(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (response) => {
        // Follow redirects (GitHub releases redirect to S3)
        if (
          response.statusCode >= 300 &&
          response.statusCode < 400 &&
          response.headers.location
        ) {
          return download(response.headers.location).then(resolve, reject);
        }

        if (response.statusCode !== 200) {
          reject(
            new Error(
              `Download failed with status ${response.statusCode}: ${url}`
            )
          );
          return;
        }

        const chunks = [];
        response.on("data", (chunk) => chunks.push(chunk));
        response.on("end", () => resolve(Buffer.concat(chunks)));
        response.on("error", reject);
      })
      .on("error", reject);
  });
}

async function main() {
  const packageJson = JSON.parse(
    fs.readFileSync(path.join(__dirname, "..", "package.json"), "utf8")
  );
  const version = packageJson.version;
  const archive = getArchiveName();
  const binDir = path.join(__dirname, "..", "bin");
  const tarballPath = path.join(binDir, `_${archive}`);

  const url = `https://github.com/pablocalofatti/cortexmem/releases/download/v${version}/${archive}`;

  console.log(`Downloading cortexmem v${version} for ${process.platform}-${process.arch}...`);
  console.log(`  ${url}`);

  const data = await download(url);
  fs.writeFileSync(tarballPath, data);

  console.log("Extracting...");
  execFileSync("tar", ["xzf", tarballPath, "-C", binDir]);

  // Clean up tarball
  fs.unlinkSync(tarballPath);

  // Ensure binary is executable
  const binaryName =
    process.platform === "win32" ? "cortexmem.exe" : "cortexmem";
  const binaryPath = path.join(binDir, binaryName);
  fs.chmodSync(binaryPath, 0o755);

  console.log(`cortexmem v${version} installed successfully.`);
}

main().catch((error) => {
  console.error(`Failed to install cortexmem: ${error.message}`);
  process.exit(1);
});
