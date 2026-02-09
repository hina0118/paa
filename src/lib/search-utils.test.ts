import { describe, it, expect } from 'vitest';
import {
  escapeFts5Query,
  buildFts5ItemBrandQuery,
  escapeLikePrefix,
} from './search-utils';

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

describe('buildFts5ItemBrandQuery', () => {
  it('returns empty string for empty or whitespace-only input', () => {
    expect(buildFts5ItemBrandQuery('')).toBe('');
    expect(buildFts5ItemBrandQuery('   ')).toBe('');
  });

  it('generates column-prefixed query for item_name and brand only', () => {
    expect(buildFts5ItemBrandQuery('商品')).toBe(
      '(item_name:("商品") OR brand:("商品"))'
    );
  });

  it('handles multiple tokens with AND', () => {
    expect(buildFts5ItemBrandQuery('RG ガンダム')).toBe(
      '(item_name:("RG" AND "ガンダム") OR brand:("RG" AND "ガンダム"))'
    );
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
