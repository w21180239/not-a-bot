#!/usr/bin/env node
'use strict';

// Thin shim: the native binary is fetched by scripts/install.js at install
// time, so this package stays platform-neutral and a few kilobytes.

const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const exe = path.join(
  __dirname,
  process.platform === 'win32' ? 'translatex.exe' : 'translatex'
);

if (!fs.existsSync(exe)) {
  process.stderr.write(
    [
      'translatex: the native binary is missing from this install.',
      '',
      'Reinstall it:',
      '  npm install -g translatex --force',
      '',
      'Or build from source:',
      '  cargo install translatex --locked',
      '',
      'If your platform has no prebuilt binary yet, use cargo.',
      '',
    ].join('\n')
  );
  process.exit(1);
}

const child = spawn(exe, process.argv.slice(2), { stdio: 'inherit' });

child.on('error', (err) => {
  process.stderr.write(`translatex: failed to start ${exe}: ${err.message}\n`);
  process.exit(1);
});

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code === null ? 1 : code);
});
