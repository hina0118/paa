import { expect, afterEach, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import * as matchers from '@testing-library/jest-dom/matchers';

// Extend Vitest's expect with jest-dom matchers
expect.extend(matchers);

// Mock ResizeObserver (used by TanStack Virtual in Orders)
// コンストラクタとして new されるため、通常の function を使用
global.ResizeObserver = vi.fn().mockImplementation(function () {
  return {
    observe: vi.fn(),
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

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: mockListen,
  emit: mockEmit,
}));

// Export mocks for use in tests
export { mockInvoke, mockListen, mockEmit };
