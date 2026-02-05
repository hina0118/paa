/**
 * WebdriverIO 設定: Tauri アプリを起動して E2E テスト
 *
 * 前提条件:
 *   - cargo install tauri-driver --locked
 *   - Windows: msedgedriver を Edge のバージョンに合わせて用意し、
 *     PATH に通すか、環境変数 MSEDGEDRIVER_PATH に msedgedriver.exe のフルパスを設定
 *     （詳細は docs/E2E_TESTING.md の「Windows: msedgedriver の用意」を参照）
 *
 * 実行: npm run test:e2e:tauri
 */

import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';
import { spawn, spawnSync } from 'child_process';
import os from 'os';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = __dirname;

const isWindows = process.platform === 'win32';
const tauriBinaryName = isWindows ? 'paa.exe' : 'paa';
const tauriAppPath = path.resolve(
  rootDir,
  'src-tauri',
  'target',
  'debug',
  tauriBinaryName
);

const cargoBin = path.join(os.homedir(), '.cargo', 'bin');
const tauriDriverPath = path.join(
  cargoBin,
  isWindows ? 'tauri-driver.exe' : 'tauri-driver'
);

let tauriDriver: ReturnType<typeof spawn> | null = null;
let exitRequested = false;

function closeTauriDriver() {
  exitRequested = true;
  if (tauriDriver) {
    tauriDriver.kill();
    tauriDriver = null;
  }
}

function onShutdown(fn: () => void) {
  const cleanup = () => {
    try {
      fn();
    } catch {
      // ignore cleanup errors on shutdown
    }
  };

  const cleanupAndExit = () => {
    cleanup();
    process.exit();
  };

  process.on('exit', cleanup);
  process.on('SIGINT', cleanupAndExit);
  process.on('SIGTERM', cleanupAndExit);
  process.on('SIGHUP', cleanupAndExit);
  if (process.platform === 'win32') {
    process.on('SIGBREAK', cleanupAndExit);
  }
}

onShutdown(() => closeTauriDriver());

export const config = {
  hostname: '127.0.0.1',
  port: 4444,
  path: '/',
  specs: ['./tests/e2e-tauri/**/*.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      'tauri:options': {
        application: tauriAppPath,
      },
    },
  ],
  reporters: ['spec'],
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 60_000,
  },

  async onPrepare() {
    const coverageEnabled = process.env.PAA_E2E_COVERAGE === '1';
    const buildEnv = { ...process.env };
    if (coverageEnabled) {
      const coverageDir = path.join(rootDir, 'coverage-e2e-tauri');
      fs.mkdirSync(coverageDir, { recursive: true });
      console.log('Coverage enabled: profraw output ->', coverageDir);
      // カバレッジ計測のため RUSTFLAGS を明示的に渡す（CI/ローカル両対応）
      buildEnv.RUSTFLAGS = process.env.RUSTFLAGS || '-Cinstrument-coverage';
    }
    console.log('Building Tauri app (debug, no bundle)...');
    const result = spawnSync(
      'npm',
      ['run', 'tauri', 'build', '--', '--debug', '--no-bundle'],
      {
        cwd: rootDir,
        stdio: 'inherit',
        shell: isWindows,
        env: buildEnv,
      }
    );
    if (result.status !== 0) {
      throw new Error(`Tauri build failed with exit code ${result.status}`);
    }
  },

  async beforeSession() {
    // Windows: msedgedriver のパスを指定する場合（PATH に通していないとき）
    const nativeDriverPath = process.env.MSEDGEDRIVER_PATH;
    const tauriDriverArgs = nativeDriverPath
      ? ['--native-driver', nativeDriverPath]
      : [];
    // 外部API（Gmail, Gemini, SerpApi）をモックに置き換える
    const env: NodeJS.ProcessEnv = { ...process.env, PAA_E2E_MOCK: '1' };
    if (process.env.PAA_E2E_COVERAGE === '1') {
      const profrawPath = path.join(
        rootDir,
        'coverage-e2e-tauri',
        'profraw-%p.profraw'
      );
      env.LLVM_PROFILE_FILE = profrawPath;
    }
    tauriDriver = spawn(tauriDriverPath, tauriDriverArgs, {
      stdio: ['ignore', process.stdout, process.stderr],
      env,
    });
    tauriDriver.on('error', (err) => {
      console.error('tauri-driver error:', err);
      process.exit(1);
    });
    tauriDriver.on('exit', (code) => {
      if (!exitRequested && code !== 0 && code !== null) {
        console.error('tauri-driver exited with code:', code);
        process.exit(1);
      }
    });
    // tauri-driver の起動を待つ
    await new Promise((resolve) => setTimeout(resolve, 3000));
  },

  async afterSession() {
    closeTauriDriver();
  },
};
