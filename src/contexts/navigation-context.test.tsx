import { describe, it, expect } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { NavigationProvider } from './navigation-provider';
import { useNavigation } from './use-navigation';
import { ReactNode } from 'react';

describe('NavigationContext', () => {
  const wrapper = ({ children }: { children: ReactNode }) => (
    <NavigationProvider>{children}</NavigationProvider>
  );

  it('provides initial screen as orders', () => {
    const { result } = renderHook(() => useNavigation(), { wrapper });
    expect(result.current.currentScreen).toBe('orders');
  });

  it('allows changing to dashboard screen', () => {
    const { result } = renderHook(() => useNavigation(), { wrapper });

    act(() => {
      result.current.setCurrentScreen('dashboard');
    });

    expect(result.current.currentScreen).toBe('dashboard');
  });

  it('allows changing to batch screen', () => {
    const { result } = renderHook(() => useNavigation(), { wrapper });

    act(() => {
      result.current.setCurrentScreen('batch');
    });

    expect(result.current.currentScreen).toBe('batch');
  });

  it('allows changing to settings screen', () => {
    const { result } = renderHook(() => useNavigation(), { wrapper });

    act(() => {
      result.current.setCurrentScreen('settings');
    });

    expect(result.current.currentScreen).toBe('settings');
  });

  it('allows changing to table screens', () => {
    const { result } = renderHook(() => useNavigation(), { wrapper });

    const tableScreens = [
      'table-emails',
      'table-orders',
      'table-items',
      'table-images',
      'table-deliveries',
      'table-htmls',
      'table-order-emails',
      'table-order-htmls',
    ] as const;

    tableScreens.forEach((screen) => {
      act(() => {
        result.current.setCurrentScreen(screen);
      });
      expect(result.current.currentScreen).toBe(screen);
    });
  });

  it('allows multiple screen changes', () => {
    const { result } = renderHook(() => useNavigation(), { wrapper });

    act(() => {
      result.current.setCurrentScreen('dashboard');
    });
    expect(result.current.currentScreen).toBe('dashboard');

    act(() => {
      result.current.setCurrentScreen('batch');
    });
    expect(result.current.currentScreen).toBe('batch');

    act(() => {
      result.current.setCurrentScreen('settings');
    });
    expect(result.current.currentScreen).toBe('settings');
  });

  it('throws error when used outside provider', () => {
    // エラーメッセージをキャプチャするためにconsole.errorをモック
    const originalError = console.error;
    console.error = () => {};

    expect(() => {
      renderHook(() => useNavigation());
    }).toThrow('useNavigation must be used within a NavigationProvider');

    console.error = originalError;
  });

  it('maintains state across re-renders', () => {
    const { result, rerender } = renderHook(() => useNavigation(), { wrapper });

    act(() => {
      result.current.setCurrentScreen('dashboard');
    });

    rerender();

    expect(result.current.currentScreen).toBe('dashboard');
  });

  it('provides both currentScreen and setCurrentScreen', () => {
    const { result } = renderHook(() => useNavigation(), { wrapper });

    expect(result.current).toHaveProperty('currentScreen');
    expect(result.current).toHaveProperty('setCurrentScreen');
    expect(typeof result.current.setCurrentScreen).toBe('function');
  });
});
