import { expect, afterEach, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import * as matchers from '@testing-library/jest-dom/matchers';

// Extend Vitest's expect with jest-dom matchers
expect.extend(matchers);

// Mock ResizeObserver (used by TanStack Virtual in Orders)
// コンストラクタとして new されるため、通常の function を使用
// 仮想スクロールが項目を描画するよう、observe 時にコールバックを実行
global.ResizeObserver = vi.fn().mockImplementation(function (
  this: ResizeObserver,
  callback: ResizeObserverCallback
) {
  return {
    observe: vi.fn((target: Element) => {
      Object.defineProperty(target, 'clientWidth', {
        value: 600,
        configurable: true,
      });
      Object.defineProperty(target, 'clientHeight', {
        value: 400,
        configurable: true,
      });
      callback(
        [
          {
            target,
            contentRect: { width: 600, height: 400 } as DOMRectReadOnly,
            borderBoxSize: [],
            contentBoxSize: [],
            devicePixelContentBoxSize: [],
          },
        ],
        this
      );
    }),
    unobserve: vi.fn(),
    disconnect: vi.fn(),
  };
});

// Cleanup after each test
afterEach(() => {
  cleanup();
});

// Mock Tauri APIs
const mockInvoke = vi.fn();
const mockListen = vi.fn();
const mockEmit = vi.fn();
const mockConvertFileSrc = vi.fn((path: string) => `asset://${path}`);
const mockIsTauri = vi.fn(() => false);

vi.mock('@tauri-apps/api/core', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tauri-apps/api/core')>();
  return {
    ...actual,
    invoke: mockInvoke,
    convertFileSrc: mockConvertFileSrc,
    isTauri: mockIsTauri,
  };
});

vi.mock('@tauri-apps/api/event', () => ({
  listen: mockListen,
  emit: mockEmit,
}));

// Export mocks for use in tests
export { mockInvoke, mockListen, mockEmit };
