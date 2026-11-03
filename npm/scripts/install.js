#!/usr/bin/env node
'use strict';

// Downloads the prebuilt binary for this platform from the matching GitHub
// release. Falls back with a clear message when no asset exists, so the package
// never installs a broken shim silently.

const fs = require('node:fs');
const path = require('node:path');
const https = require('node:https');

const pkg = require('../package.json');
const REPO = 'w21180239/translatex';

const TARGETS = {
  'darwin-x64': 'translatex-x86_64-apple-darwin',
  'darwin-arm64': 'translatex-aarch64-apple-darwin',
  'linux-x64': 'translatex-x86_64-unknown-linux-gnu',
  'linux-arm64': 'translatex-aarch64-unknown-linux-gnu',
  'win32-x64': 'translatex-x86_64-pc-windows-msvc',
};

function target() {
  const key = `${process.platform}-${process.arch}`;
  const name = TARGETS[key];
  if (!name) {
    return null;
  }
  return name;
}

function download(url, destination, redirects = 0) {
  return new Promise((resolve, reject) => {
    if (redirects > 5) {
      reject(new Error(`too many redirects for ${url}`));
      return;
    }
    https
      .get(url, { headers: { 'user-agent': `translatex-npm/${pkg.version}` } }, (res) => {
        const { statusCode, headers } = res;
        if (statusCode >= 300 && statusCode < 400 && headers.location) {
          res.resume();
          download(headers.location, destination, redirects + 1).then(resolve, reject);
          return;
        }
        if (statusCode !== 200) {
          res.resume();
          reject(new Error(`HTTP ${statusCode} for ${url}`));
          return;
        }
        const out = fs.createWriteStream(destination);
        res.pipe(out);
        out.on('finish', () => out.close(resolve));
        out.on('error', reject);
      })
      .on('error', reject);
  });
}

async function main() {
  // Escape hatch for air-gapped installs and for trying a local build.
  if (process.env.TRANSLATEX_BINARY_PATH) {
    const from = process.env.TRANSLATEX_BINARY_PATH;
    const to = path.join(__dirname, '..', 'bin', path.basename(from));
    fs.copyFileSync(from, to);
    process.stderr.write(`translatex: using ${from}\n`);
    return;
  }

  const name = target();
  if (!name) {
    process.stderr.write(
      [
        `translatex: no prebuilt binary for ${process.platform}/${process.arch}.`,
        '',
        'Install from source instead:',
        '  cargo install translatex --locked',
        '',
      ].join('\n')
    );
    process.exit(1);
  }

  const exe = process.platform === 'win32' ? 'translatex.exe' : 'translatex';
  const destination = path.join(__dirname, '..', 'bin', exe);

  if (fs.existsSync(destination)) {
    fs.chmodSync(destination, 0o755);
    return;
  }

  const base = process.env.TRANSLATEX_RELEASE_BASE
    || `https://github.com/${REPO}/releases/download/v${pkg.version}`;
  const url = `${base}/${name}.tar.gz`;
  const tmp = `${destination}.tar.gz`;

  try {
    process.stderr.write(`translatex: downloading ${url}\n`);
    await download(url, tmp);
    // The release asset is a .tar.gz holding the binary at its root.
    await new Promise((resolve, reject) => {
      const { execFile } = require('node:child_process');
      execFile('tar', ['-xzf', tmp, '-C', path.dirname(destination)], (err) => {
        if (err) reject(err);
        else resolve();
      });
    });
    fs.rmSync(tmp, { force: true });
    fs.chmodSync(destination, 0o755);
    process.stderr.write(`translatex: installed ${destination}\n`);
  } catch (err) {
    fs.rmSync(tmp, { force: true });
    process.stderr.write(
      [
        `translatex: could not download the prebuilt binary (${err.message}).`,
        '',
        'Install from source instead:',
        '  cargo install translatex --locked',
        '',
      ].join('\n')
    );
    process.exit(1);
  }
}

main();
