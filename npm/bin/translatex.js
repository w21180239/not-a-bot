#!/usr/bin/env node
'use strict';

// The native binary ships in a per-platform package declared as an optional
// dependency, so nothing runs at install time. That matters because npm 12
// blocks dependency install scripts by default: a download-on-install shim
// would silently install nothing.

const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const exe = process.platform === 'win32' ? 'translatex.exe' : 'translatex';

// npm blocks package names that look machine-generated, and "win32" trips its
// spam filter, so the Windows package is published as translatex-windows-x64.
const platformName = process.platform === 'win32' ? 'windows' : process.platform;
const platformPackage = `translatex-${platformName}-${process.arch}`;

function binaryFromPlatformPackage() {
  let packageJson;
  try {
    packageJson = require.resolve(`${platformPackage}/package.json`);
  } catch {
    return null;
  }
  const candidate = path.join(path.dirname(packageJson), exe);
  return fs.existsSync(candidate) ? candidate : null;
}

const binary = process.env.TRANSLATEX_BINARY_PATH || binaryFromPlatformPackage();

if (!binary) {
  process.stderr.write(
    [
      `translatex: no binary for ${process.platform}/${process.arch}.`,
      '',
      `The package ${platformPackage} is missing. Reinstall, or install from source:`,
      '  npm install -g translatex --force',
      '  cargo install translatex --locked',
      '',
      'If your platform has no prebuilt binary, cargo is the way.',
      '',
    ].join('\n')
  );
  process.exit(1);
}

const child = spawn(binary, process.argv.slice(2), { stdio: 'inherit' });

child.on('error', (err) => {
  process.stderr.write(`translatex: failed to start ${binary}: ${err.message}\n`);
  process.exit(1);
});

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code === null ? 1 : code);
});
