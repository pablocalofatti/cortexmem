#!/usr/bin/env node

"use strict";

const { execFileSync } = require("child_process");
const path = require("path");

const binaryName = process.platform === "win32" ? "cortexmem.exe" : "cortexmem";
const binaryPath = path.join(__dirname, binaryName);

try {
  execFileSync(binaryPath, process.argv.slice(2), {
    stdio: "inherit",
  });
} catch (error) {
  if (error.status !== null) {
    process.exit(error.status);
  }
  console.error(`Failed to run cortexmem: ${error.message}`);
  process.exit(1);
}
