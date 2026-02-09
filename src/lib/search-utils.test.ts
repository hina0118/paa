import { describe, it, expect } from 'vitest';
import { escapeFts5Query, escapeLikePrefix } from './search-utils';

describe('escapeFts5Query', () => {
  it('returns empty string for empty or whitespace-only input', () => {
    expect(escapeFts5Query('')).toBe('');
    expect(escapeFts5Query('   ')).toBe('');
  });

  it('wraps single token in quotes', () => {
    expect(escapeFts5Query('商品')).toBe('"商品"');
    expect(escapeFts5Query('ガンダム')).toBe('"ガンダム"');
  });

  it('joins multiple tokens with AND', () => {
    expect(escapeFts5Query('RG ガンダム')).toBe('"RG" AND "ガンダム"');
  });

  it('escapes double quotes in tokens', () => {
    expect(escapeFts5Query('It\'s "quoted"')).toBe('"It\'s" AND """quoted"""');
  });
});

describe('escapeLikePrefix', () => {
  it('returns empty string for empty or whitespace-only input', () => {
    expect(escapeLikePrefix('')).toBe('');
    expect(escapeLikePrefix('   ')).toBe('');
  });

  it('returns trimmed string for simple input', () => {
    expect(escapeLikePrefix('1999')).toBe('1999');
  });

  it('escapes % and _ for LIKE', () => {
    expect(escapeLikePrefix('50%')).toBe('50\\%');
    expect(escapeLikePrefix('A_B')).toBe('A\\_B');
  });

  it('escapes backslash', () => {
    expect(escapeLikePrefix('path\\to')).toBe('path\\\\to');
  });
});
