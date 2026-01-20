import { describe, it, expect } from 'vitest';
import { cn } from './utils';

describe('cn utility', () => {
  it('merges class names correctly', () => {
    const result = cn('text-red-500', 'bg-blue-500');
    expect(result).toBe('text-red-500 bg-blue-500');
  });

  it('handles conditional classes', () => {
    const isActive = true;
    const isHidden = false;
    const result = cn(
      'base-class',
      isActive && 'conditional-class',
      isHidden && 'hidden-class'
    );
    expect(result).toBe('base-class conditional-class');
  });

  it('merges Tailwind classes correctly', () => {
    // Tailwind merge should resolve conflicting classes
    const result = cn('px-2 py-1', 'px-4');
    expect(result).toBe('py-1 px-4');
  });

  it('handles undefined and null', () => {
    const result = cn('base', undefined, null, 'other');
    expect(result).toBe('base other');
  });

  it('handles empty input', () => {
    const result = cn();
    expect(result).toBe('');
  });

  it('handles arrays of classes', () => {
    const result = cn(['class1', 'class2'], 'class3');
    expect(result).toBe('class1 class2 class3');
  });

  it('handles objects with conditional classes', () => {
    const result = cn({
      'base-class': true,
      hidden: false,
      active: true,
    });
    expect(result).toContain('base-class');
    expect(result).toContain('active');
    expect(result).not.toContain('hidden');
  });
});
