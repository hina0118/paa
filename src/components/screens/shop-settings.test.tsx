import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Toaster } from 'sonner';
import { ShopSettings } from './shop-settings';
import { mockInvoke } from '@/test/setup';

const renderComponent = () =>
  render(
    <>
      <ShopSettings />
      <Toaster position="top-right" richColors />
    </>
  );

const mockShop = {
  id: 1,
  shop_name: 'テスト店舗',
  sender_address: 'test@example.com',
  parser_type: 'TypeA',
  is_enabled: true,
  subject_filters: null,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
};

describe('ShopSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_all_shop_settings') {
        return Promise.resolve([mockShop]);
      }
      return Promise.resolve(null);
    });
  });

  it('renders the shop settings heading', async () => {
    renderComponent();
    await waitFor(() => {
      expect(
        screen.getByRole('heading', { name: /店舗設定/i })
      ).toBeInTheDocument();
    });
  });

  it('displays a shop group card after loading', async () => {
    renderComponent();
    await waitFor(() => {
      expect(screen.getByText('テスト店舗')).toBeInTheDocument();
    });
  });

  describe('is_enabled checkbox in edit mode', () => {
    it('calls update_shop_setting with isEnabled: false when checkbox is unchecked and saved', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_all_shop_settings') {
          return Promise.resolve([mockShop]);
        }
        if (cmd === 'update_shop_setting') {
          return Promise.resolve(undefined);
        }
        return Promise.resolve(null);
      });

      renderComponent();

      // Wait for shop group to appear and expand it
      await waitFor(() => {
        expect(screen.getByText('テスト店舗')).toBeInTheDocument();
      });

      // Click the group-level expand button (aria-expanded=false)
      const expandButton = screen.getByRole('button', {
        name: /編集/,
        expanded: false,
      });
      await user.click(expandButton);

      // Click the row-level edit button inside the expanded group
      await waitFor(() => {
        expect(
          screen.getAllByRole('button', { name: '編集' }).length
        ).toBeGreaterThan(0);
      });
      const rowEditButtons = screen.getAllByRole('button', { name: '編集' });
      await user.click(rowEditButtons[rowEditButtons.length - 1]);

      // The edit form should appear with the is_enabled checkbox (checked by default)
      await waitFor(() => {
        expect(screen.getByRole('checkbox')).toBeInTheDocument();
      });
      const checkbox = screen.getByRole('checkbox');
      expect(checkbox).toBeChecked();

      // Uncheck the checkbox
      await user.click(checkbox);
      expect(checkbox).not.toBeChecked();

      // Save
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          'update_shop_setting',
          expect.objectContaining({ isEnabled: false })
        );
      });
    });

    it('calls update_shop_setting with isEnabled: true when checkbox remains checked', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_all_shop_settings') {
          return Promise.resolve([mockShop]);
        }
        if (cmd === 'update_shop_setting') {
          return Promise.resolve(undefined);
        }
        return Promise.resolve(null);
      });

      renderComponent();

      await waitFor(() => {
        expect(screen.getByText('テスト店舗')).toBeInTheDocument();
      });

      const expandButton = screen.getByRole('button', {
        name: /編集/,
        expanded: false,
      });
      await user.click(expandButton);

      await waitFor(() => {
        expect(
          screen.getAllByRole('button', { name: '編集' }).length
        ).toBeGreaterThan(0);
      });
      const rowEditButtons = screen.getAllByRole('button', { name: '編集' });
      await user.click(rowEditButtons[rowEditButtons.length - 1]);

      await waitFor(() => {
        expect(screen.getByRole('checkbox')).toBeInTheDocument();
      });
      const checkbox = screen.getByRole('checkbox');
      expect(checkbox).toBeChecked();

      // Leave checkbox checked and save
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          'update_shop_setting',
          expect.objectContaining({ isEnabled: true })
        );
      });
    });
  });

  describe('shop group enable/disable buttons', () => {
    it('calls toggle_shop_enabled with isEnabled: false when clicking the disable button for an enabled shop', async () => {
      const user = userEvent.setup();

      mockInvoke.mockImplementation((cmd: string, args?: unknown) => {
        if (cmd === 'get_all_shop_settings') {
          return Promise.resolve([mockShop]);
        }
        if (cmd === 'toggle_shop_enabled') {
          return Promise.resolve(undefined);
        }
        return Promise.resolve(null);
      });

      renderComponent();

      await waitFor(() => {
        expect(screen.getByText('テスト店舗')).toBeInTheDocument();
      });

      const disableButton = screen.getByRole('button', { name: '無効化' });
      await user.click(disableButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          'toggle_shop_enabled',
          expect.objectContaining({
            shopName: mockShop.shop_name,
            isEnabled: false,
          })
        );
      });
    });

    it('calls toggle_shop_enabled with isEnabled: true when clicking the enable button for a disabled shop', async () => {
      const user = userEvent.setup();

      const disabledShop = { ...mockShop, is_enabled: false };

      mockInvoke.mockImplementation((cmd: string, args?: unknown) => {
        if (cmd === 'get_all_shop_settings') {
          return Promise.resolve([disabledShop]);
        }
        if (cmd === 'toggle_shop_enabled') {
          return Promise.resolve(undefined);
        }
        return Promise.resolve(null);
      });

      renderComponent();

      await waitFor(() => {
        expect(screen.getByText('テスト店舗')).toBeInTheDocument();
      });

      const enableButton = screen.getByRole('button', { name: '有効化' });
      await user.click(enableButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          'toggle_shop_enabled',
          expect.objectContaining({
            shopName: mockShop.shop_name,
            isEnabled: true,
          })
        );
      });
    });
  });
});
