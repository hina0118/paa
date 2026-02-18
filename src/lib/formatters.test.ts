import { describe, it, expect } from 'vitest';
import {
  formatNumber,
  formatBytes,
  formatCharacters,
  formatCurrency,
  calculatePercentage,
} from './formatters';

describe('formatNumber', () => {
  it('formats integer with ja-JP locale', () => {
    expect(formatNumber(1000)).toBe('1,000');
  });

  it('formats zero', () => {
    expect(formatNumber(0)).toBe('0');
  });

  it('formats large number', () => {
    expect(formatNumber(1234567)).toBe('1,234,567');
  });
});

describe('formatBytes', () => {
  it('delegates to formatCharacters (deprecated)', () => {
    expect(formatBytes(0)).toBe(formatCharacters(0));
    expect(formatBytes(500)).toBe(formatCharacters(500));
    expect(formatBytes(500.7)).toBe(formatCharacters(500.7));
    expect(formatBytes(2000)).toBe(formatCharacters(2000));
  });
});

describe('formatCharacters', () => {
  it('returns "0 文字" for 0', () => {
    expect(formatCharacters(0)).toBe('0 文字');
  });

  it('formats positive number with 文字 suffix', () => {
    expect(formatCharacters(500)).toBe('500 文字');
  });

  it('rounds decimal values', () => {
    expect(formatCharacters(500.7)).toBe('501 文字');
  });

  it('formats large value with comma', () => {
    expect(formatCharacters(2000)).toBe('2,000 文字');
  });
});

describe('formatCurrency', () => {
  it('prepends ¥ to formatted number', () => {
    expect(formatCurrency(1500)).toBe('¥1,500');
  });

  it('formats zero', () => {
    expect(formatCurrency(0)).toBe('¥0');
  });

  it('formats large amount', () => {
    expect(formatCurrency(150000)).toBe('¥150,000');
  });
});

describe('calculatePercentage', () => {
  it('returns "0" when total is 0', () => {
    expect(calculatePercentage(0, 0)).toBe('0');
    expect(calculatePercentage(5, 0)).toBe('0');
  });

  it('calculates percentage to 1 decimal place', () => {
    expect(calculatePercentage(80, 100)).toBe('80.0');
    expect(calculatePercentage(1, 3)).toBe('33.3');
  });

  it('returns "100.0" when part equals total', () => {
    expect(calculatePercentage(100, 100)).toBe('100.0');
  });
});
