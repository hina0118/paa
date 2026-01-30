import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetFooter,
  SheetTitle,
  SheetDescription,
} from './sheet';

describe('Sheet Components', () => {
  describe('SheetContent', () => {
    it('renders with default side right', () => {
      render(
        <Sheet open>
          <SheetContent>
            <p>Content</p>
          </SheetContent>
        </Sheet>
      );
      expect(screen.getByText('Content')).toBeInTheDocument();
      const content = screen.getByText('Content').closest('.border-l');
      expect(content?.className).toContain('right-0');
    });

    it('renders with side top', () => {
      render(
        <Sheet open>
          <SheetContent side="top">
            <p>Top content</p>
          </SheetContent>
        </Sheet>
      );
      expect(screen.getByText('Top content')).toBeInTheDocument();
      const content = screen.getByText('Top content').closest('.border-b');
      expect(content?.className).toContain('top-0');
    });

    it('renders with side bottom', () => {
      render(
        <Sheet open>
          <SheetContent side="bottom">
            <p>Bottom content</p>
          </SheetContent>
        </Sheet>
      );
      expect(screen.getByText('Bottom content')).toBeInTheDocument();
      const content = screen.getByText('Bottom content').closest('.border-t');
      expect(content?.className).toContain('bottom');
    });

    it('renders with side left', () => {
      render(
        <Sheet open>
          <SheetContent side="left">
            <p>Left content</p>
          </SheetContent>
        </Sheet>
      );
      expect(screen.getByText('Left content')).toBeInTheDocument();
      const content = screen.getByText('Left content').closest('.border-r');
      expect(content?.className).toContain('left-0');
    });
  });

  describe('SheetHeader', () => {
    it('renders header content', () => {
      render(
        <Sheet open>
          <SheetContent>
            <SheetHeader>Header text</SheetHeader>
          </SheetContent>
        </Sheet>
      );
      expect(screen.getByText('Header text')).toBeInTheDocument();
    });
  });

  describe('SheetFooter', () => {
    it('renders footer content', () => {
      render(
        <Sheet open>
          <SheetContent>
            <SheetFooter>Footer text</SheetFooter>
          </SheetContent>
        </Sheet>
      );
      expect(screen.getByText('Footer text')).toBeInTheDocument();
    });
  });

  describe('SheetTitle', () => {
    it('renders title', () => {
      render(
        <Sheet open>
          <SheetContent>
            <SheetTitle>Sheet Title</SheetTitle>
          </SheetContent>
        </Sheet>
      );
      expect(screen.getByText('Sheet Title')).toBeInTheDocument();
    });
  });

  describe('SheetDescription', () => {
    it('renders description', () => {
      render(
        <Sheet open>
          <SheetContent>
            <SheetDescription>Sheet description</SheetDescription>
          </SheetContent>
        </Sheet>
      );
      expect(screen.getByText('Sheet description')).toBeInTheDocument();
    });
  });
});
