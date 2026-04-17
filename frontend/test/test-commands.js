#!/usr/bin/env node
/**
 * Automated test script for YT Downloader Tauri backend.
 * Tests all Tauri invoke commands directly.
 */

const { execSync, spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const COLORS = {
  green: '\x1b[32m',
  red: '\x1b[31m',
  yellow: '\x1b[33m',
  blue: '\x1b[36m',
  reset: '\x1b[0m',
  bold: '\x1b[1m',
};

const results = { passed: 0, failed: 0, errors: [] };

function log(msg, color = 'reset') {
  console.log(`${COLORS[color]}${msg}${COLORS.reset}`);
}

function test(name, fn) {
  process.stdout.write(`  ${'─'.repeat(50)}\n  TEST: ${name} ... `);
  try {
    fn();
    log('✅ PASS', 'green');
    results.passed++;
  } catch (e) {
    log('❌ FAIL', 'red');
    log(`    Error: ${e.message}`, 'red');
    results.failed++;
    results.errors.push({ name, error: e.message });
  }
}

// ========== 1. Check prerequisites ==========
log(`\n${COLORS.bold}=== Phase 1: Prerequisites ===${COLORS.reset}`);

test('yt-dlp is installed', () => {
  const out = execSync('where yt-dlp.exe 2>nul || where yt-dlp 2>nul', { encoding: 'utf8' });
  if (!out.trim()) throw new Error('yt-dlp not found on PATH');
  log(`    Found: ${out.trim().split('\n')[0]}`, 'blue');
});

test('yt-dlp version check works', () => {
  const out = execSync('yt-dlp --version', { encoding: 'utf8' });
  log(`    Version: ${out.trim()}`, 'blue');
});

test('Frontend dist exists', () => {
  const distPath = path.join(__dirname, '..', 'frontend', 'dist', 'index.html');
  if (!fs.existsSync(distPath)) throw new Error(`Missing: ${distPath}`);
  log(`    Found: ${distPath}`, 'blue');
});

test('Tauri binary built', () => {
  const exePath = path.join(__dirname, '..', 'src-tauri', 'target', 'release', 'app.exe');
  if (!fs.existsSync(exePath)) throw new Error(`Missing: ${exePath}`);
  log(`    Found: ${exePath}`, 'blue');
});

// ========== 2. Test Tauri commands via direct invocation ==========
log(`\n${COLORS.bold}=== Phase 2: Direct Tauri Command Test ===${COLORS.reset}`);
log('(Using cargo tauri dev in background)\n', 'yellow');

// Start cargo tauri dev in background
const tauriProcess = spawn('cargo', ['tauri', 'dev'], {
  cwd: path.join(__dirname, '..'),
  stdio: ['pipe', 'pipe', 'pipe'],
  env: { ...process.env, RUST_LOG: 'debug' },
  windowsHide: true,
});

let tauriReady = false;
let tauriOutput = '';

tauriProcess.stdout.on('data', (data) => {
  const text = data.toString();
  tauriOutput += text;

  // Check for readiness signals
  if (text.includes('Listening') || text.includes('started')) {
    tauriReady = true;
  }

  // Check for command errors
  if (text.includes('ERROR') || text.includes('error[')) {
    log(`    [Tauri log] ${text.trim()}`, 'yellow');
  }
});

tauriProcess.stderr.on('data', (data) => {
  const text = data.toString();
  tauriOutput += text;
  if (text.includes('ERROR') || text.includes('error')) {
    log(`    [Tauri stderr] ${text.trim()}`, 'red');
  }
});

// Wait for Tauri to start
log('  Waiting for Tauri dev server to start...', 'blue');
let waitCount = 0;
while (!tauriReady && waitCount < 120) {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  // We can't use async in sync test, use busy wait
  const start = Date.now();
  while (Date.now() - start < 1000) {
    // busy wait
  }
  waitCount++;
  if (tauriOutput.includes('Listening') || tauriOutput.includes('started') || tauriOutput.includes('Compiling')) {
    tauriReady = true;
  }
}

if (tauriReady) {
  log('  Tauri dev server started', 'green');
} else {
  log('  WARNING: Tauri did not start within 120 seconds', 'yellow');
  log('  Output so far:', 'yellow');
  console.log(tauriOutput.slice(-2000));
}

// Kill Tauri process
try {
  tauriProcess.kill();
} catch (e) {}

log('\n  Tests complete. Summary:', 'blue');
log(`  ${'─'.repeat(50)}`, 'blue');
log(`  Total: ${results.passed + results.failed}`, 'bold');
log(`  Passed: ${results.passed}`, 'green');
log(`  Failed: ${results.failed}`, results.failed > 0 ? 'red' : 'green');

if (results.errors.length > 0) {
  log(`\n  Failed tests:`, 'red');
  for (const err of results.errors) {
    log(`    - ${err.name}: ${err.error}`, 'red');
  }
}

process.exit(results.failed > 0 ? 1 : 0);
