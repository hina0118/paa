import { describe, it, expect } from 'vitest';
import {
  VALID_TABLES,
  isValidTableName,
  sanitizeTableName,
} from './table-utils';

describe('table-utils', () => {
  describe('VALID_TABLES', () => {
    it('contains expected table names', () => {
      expect(VALID_TABLES).toContain('emails');
      expect(VALID_TABLES).toContain('orders');
      expect(VALID_TABLES).toContain('items');
      expect(VALID_TABLES).toContain('images');
      expect(VALID_TABLES).toContain('deliveries');
      expect(VALID_TABLES).toContain('shop_settings');
      expect(VALID_TABLES).toContain('product_master');
    });
  });

  describe('isValidTableName', () => {
    it('returns true for all valid table names', () => {
      for (const name of VALID_TABLES) {
        expect(isValidTableName(name)).toBe(true);
      }
    });

    it('returns false for invalid table names', () => {
      expect(isValidTableName('invalid_table')).toBe(false);
      expect(isValidTableName('users')).toBe(false);
      expect(isValidTableName('')).toBe(false);
      expect(isValidTableName('emails; DROP TABLE emails')).toBe(false);
      expect(isValidTableName("emails' OR '1'='1")).toBe(false);
    });
  });

  describe('sanitizeTableName', () => {
    it('returns table name as-is when valid', () => {
      for (const name of VALID_TABLES) {
        expect(sanitizeTableName(name)).toBe(name);
      }
    });

    it('throws for invalid table names', () => {
      expect(() => sanitizeTableName('invalid')).toThrow(
        /Table "invalid" is not allowed/
      );
      expect(() => sanitizeTableName('invalid')).toThrow(
        /Allowed tables are: .*emails.*orders/
      );
      expect(() => sanitizeTableName('invalid')).toThrow(
        /configuration issue or a bug/
      );
    });
  });
});
