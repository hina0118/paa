import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { DatabaseManager, isTauriEnv } from './database';

describe('DatabaseManager', () => {
  beforeEach(async () => {
    await DatabaseManager.resetAsync();
  });

  afterEach(async () => {
    await DatabaseManager.resetAsync();
  });

  it('returns same instance from getInstance (singleton)', () => {
    const a = DatabaseManager.getInstance();
    const b = DatabaseManager.getInstance();
    expect(a).toBe(b);
  });

  it('returns database with select method when not in Tauri', async () => {
    const manager = DatabaseManager.getInstance();
    const db = await manager.getDatabase();
    expect(db).toBeDefined();
    expect(typeof db.select).toBe('function');
    const result = await db.select('SELECT 1');
    expect(Array.isArray(result)).toBe(true);
  });

  it('returns same db instance on multiple getDatabase calls', async () => {
    const manager = DatabaseManager.getInstance();
    const db1 = await manager.getDatabase();
    const db2 = await manager.getDatabase();
    expect(db1).toBe(db2);
  });

  it('throws when getDatabase is called after cleanup', async () => {
    const manager = DatabaseManager.getInstance();
    await manager.getDatabase();
    manager.cleanup();
    await expect(manager.getDatabase()).rejects.toThrow(
      'DatabaseManager is closing, cannot get database connection'
    );
  });

  it('resetAsync clears singleton so new instance can be created', async () => {
    const manager1 = DatabaseManager.getInstance();
    await manager1.getDatabase();
    await DatabaseManager.resetAsync();

    const manager2 = DatabaseManager.getInstance();
    expect(manager2).not.toBe(manager1);
    const db = await manager2.getDatabase();
    expect(db).toBeDefined();
  });

  it('reset clears singleton', async () => {
    const manager1 = DatabaseManager.getInstance();
    await manager1.getDatabase();
    DatabaseManager.reset();

    const manager2 = DatabaseManager.getInstance();
    expect(manager2).not.toBe(manager1);
    const db = await manager2.getDatabase();
    expect(db).toBeDefined();
  });
});

describe('isTauriEnv', () => {
  it('returns false when __TAURI__ is not set', () => {
    expect(isTauriEnv()).toBe(false);
  });

  it('returns true when __TAURI__ is set', () => {
    const original = (window as Window & { __TAURI__?: unknown }).__TAURI__;
    (window as Window & { __TAURI__?: unknown }).__TAURI__ = {};
    expect(isTauriEnv()).toBe(true);
    (window as Window & { __TAURI__?: unknown }).__TAURI__ = original;
  });
});

describe('DatabaseManager - cleanupAsync', () => {
  beforeEach(async () => {
    await DatabaseManager.resetAsync();
  });

  afterEach(async () => {
    await DatabaseManager.resetAsync();
  });

  it('resetAsync completes when db is initialized', async () => {
    const manager = DatabaseManager.getInstance();
    await manager.getDatabase();
    await expect(DatabaseManager.resetAsync()).resolves.toBeUndefined();
  });

  it('getDatabase throws when called during cleanup', async () => {
    const manager = DatabaseManager.getInstance();
    manager.cleanup();
    await expect(manager.getDatabase()).rejects.toThrow(
      'DatabaseManager is closing, cannot get database connection'
    );
  });
});

describe('DatabaseManager - reset', () => {
  beforeEach(async () => {
    await DatabaseManager.resetAsync();
  });

  it('reset clears instance when it exists', async () => {
    const manager = DatabaseManager.getInstance();
    await manager.getDatabase();
    DatabaseManager.reset();
    const manager2 = DatabaseManager.getInstance();
    expect(manager2).not.toBe(manager);
  });

  it('reset does nothing when instance is null', () => {
    DatabaseManager.reset();
    expect(() => DatabaseManager.reset()).not.toThrow();
  });
});
