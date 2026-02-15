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
import { fileURLToPath } from 'url';
import { spawn, spawnSync, execSync } from 'child_process';
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
// CI (Linux) で LLVM_PROFILE_FILE を渡すラッパー（tauri-driver は子プロセスに env を継承しない）
const coverageWrapperPath = path.resolve(
  rootDir,
  'scripts',
  'run-paa-with-coverage.sh'
);

const cargoBin = path.join(os.homedir(), '.cargo', 'bin');
const tauriDriverPath = path.join(
  cargoBin,
  isWindows ? 'tauri-driver.exe' : 'tauri-driver'
);

let tauriDriver: ReturnType<typeof spawn> | null = null;
let exitRequested = false;

function killPaaProcesses() {
  // tauri-driver が起動した paa アプリを確実に終了させる
  // afterSession で tauri-driver だけ kill しても paa プロセスが残り、
  // 次の worker で single-instance チェックに引っかかるため
  try {
    if (isWindows) {
      execSync('taskkill /F /IM paa.exe 2>nul', { stdio: 'ignore' });
    } else {
      // debug ビルドのバイナリのフルパスで特定して kill
      execSync(`pkill -f "${tauriAppPath}" 2>/dev/null || true`, {
        stdio: 'ignore',
      });
    }
  } catch {
    // プロセスが既に終了している場合は無視
  }
}

function closeTauriDriver() {
  exitRequested = true;
  if (tauriDriver) {
    tauriDriver.kill();
    tauriDriver = null;
  }
  killPaaProcesses();
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
        // カバレッジ時はラッパーを使用（LLVM_PROFILE_FILE を設定してから paa を起動）
        application:
          process.env.PAA_E2E_COVERAGE === '1' && !isWindows
            ? coverageWrapperPath
            : tauriAppPath,
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
      const profrawDir = path.join(rootDir, 'src-tauri', 'target');
      console.log(
        'Coverage enabled: profraw output ->',
        profrawDir,
        '(src-tauri-%p-%m.profraw)'
      );
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
    // 前回のセッションで残った paa プロセスを確実に終了させる
    killPaaProcesses();
    await new Promise((resolve) => setTimeout(resolve, 500));

    // Windows: msedgedriver のパスを指定する場合（PATH に通していないとき）
    const nativeDriverPath = process.env.MSEDGEDRIVER_PATH;
    const tauriDriverArgs = nativeDriverPath
      ? ['--native-driver', nativeDriverPath]
      : [];
    // 外部API（Gmail, Gemini, SerpApi）をモックに置き換える
    const env: NodeJS.ProcessEnv = { ...process.env, PAA_E2E_MOCK: '1' };
    // カバレッジ時は application にラッパースクリプトを指定（LLVM_PROFILE_FILE を設定）
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
    // paa プロセスの終了と D-Bus シングルインスタンスロックの解放を待つ
    await new Promise((resolve) => setTimeout(resolve, 1000));
  },
};
