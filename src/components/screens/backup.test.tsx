import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Toaster } from 'sonner';
import { Backup } from './backup';
import { mockInvoke } from '@/test/setup';

// Mock Tauri dialog plugin
const mockSave = vi.fn();
const mockOpen = vi.fn();
const mockConfirm = vi.fn();

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: (...args: unknown[]) => mockSave(...args),
  open: (...args: unknown[]) => mockOpen(...args),
  confirm: (...args: unknown[]) => mockConfirm(...args),
}));

const renderWithToaster = (ui: React.ReactElement) => {
  return render(
    <>
      {ui}
      <Toaster position="top-right" richColors />
    </>
  );
};

describe('Backup', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders backup heading', () => {
    renderWithToaster(<Backup />);
    expect(
      screen.getByRole('heading', { name: /データのバックアップ/i, level: 1 })
    ).toBeInTheDocument();
  });

  it('renders export card', () => {
    renderWithToaster(<Backup />);
    expect(
      screen.getByRole('heading', { name: /データのバックアップ/, level: 3 })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'データのバックアップ' })
    ).toBeInTheDocument();
  });

  it('renders import card', () => {
    renderWithToaster(<Backup />);
    expect(
      screen.getByRole('heading', { name: /データのインポート/ })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'データのインポート' })
    ).toBeInTheDocument();
  });

  it('renders restore card', () => {
    renderWithToaster(<Backup />);
    expect(
      screen.getByRole('heading', { name: /復元（復元ポイント）/ })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: '復元（復元ポイント）' })
    ).toBeInTheDocument();
  });

  describe('handleExport', () => {
    it('shows success toast with total and details when export succeeds', async () => {
      const user = userEvent.setup();
      mockSave.mockResolvedValue('/path/to/export.zip');
      mockInvoke.mockResolvedValue({
        images_count: 10,
        shop_settings_count: 2,
        product_master_count: 50,
        emails_count: 30,
        item_overrides_count: 5,
        order_overrides_count: 3,
        excluded_items_count: 1,
        excluded_orders_count: 2,
        image_files_count: 45,
        images_skipped: 0,
      });

      renderWithToaster(<Backup />);

      await user.click(
        screen.getByRole('button', { name: 'データのバックアップ' })
      );

      await waitFor(() => {
        expect(mockSave).toHaveBeenCalled();
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('export_metadata', {
          savePath: '/path/to/export.zip',
        });
      });

      // Check main message shows total (excluding image_files_count) and image files separately
      await waitFor(() => {
        expect(
          screen.getByText(
            /バックアップを保存しました（合計: 103件、画像ファイル: 45件）/
          )
        ).toBeInTheDocument();
      });

      // Check details are shown (should not include image files in details)
      await waitFor(() => {
        const detailsText = screen.getByText(/images: 10件/);
        expect(detailsText).toBeInTheDocument();
      });
    });

    it('shows warning toast when images are skipped', async () => {
      const user = userEvent.setup();
      mockSave.mockResolvedValue('/path/to/export.zip');
      mockInvoke.mockResolvedValue({
        images_count: 10,
        shop_settings_count: 2,
        product_master_count: 50,
        emails_count: 30,
        item_overrides_count: 5,
        order_overrides_count: 3,
        excluded_items_count: 1,
        excluded_orders_count: 2,
        image_files_count: 45,
        images_skipped: 5,
      });

      renderWithToaster(<Backup />);

      await user.click(
        screen.getByRole('button', { name: 'データのバックアップ' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(/5件の画像をスキップしました/)
        ).toBeInTheDocument();
      });
    });

    it('does not export when dialog is cancelled', async () => {
      const user = userEvent.setup();
      mockSave.mockResolvedValue(null);

      renderWithToaster(<Backup />);

      await user.click(
        screen.getByRole('button', { name: 'データのバックアップ' })
      );

      await waitFor(() => {
        expect(mockSave).toHaveBeenCalled();
      });

      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it('shows error toast when export fails', async () => {
      const user = userEvent.setup();
      mockSave.mockResolvedValue('/path/to/export.zip');
      mockInvoke.mockRejectedValue(new Error('Export failed'));

      renderWithToaster(<Backup />);

      await user.click(
        screen.getByRole('button', { name: 'データのバックアップ' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(/エクスポートに失敗しました/)
        ).toBeInTheDocument();
      });
    });
  });

  describe('handleImport', () => {
    it('shows success toast with total and details when import succeeds', async () => {
      const user = userEvent.setup();
      mockConfirm.mockResolvedValue(true);
      mockOpen.mockResolvedValue('/path/to/import.zip');
      mockInvoke.mockResolvedValue({
        images_inserted: 8,
        shop_settings_inserted: 1,
        product_master_inserted: 40,
        emails_inserted: 25,
        item_overrides_inserted: 4,
        order_overrides_inserted: 2,
        excluded_items_inserted: 1,
        excluded_orders_inserted: 1,
        image_files_copied: 38,
      });

      renderWithToaster(<Backup />);

      await user.click(
        screen.getByRole('button', { name: 'データのインポート' })
      );

      await waitFor(() => {
        expect(mockConfirm).toHaveBeenCalled();
      });

      await waitFor(() => {
        expect(mockOpen).toHaveBeenCalled();
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('import_metadata', {
          zipPath: '/path/to/import.zip',
        });
      });

      // Check main message shows total (excluding image_files_copied) and image files separately
      await waitFor(() => {
        expect(
          screen.getByText(
            /インポートしました（合計: 82件、画像ファイル: 38件）/
          )
        ).toBeInTheDocument();
      });

      // Check details are shown
      await waitFor(() => {
        const detailsText = screen.getByText(/images: 8件/);
        expect(detailsText).toBeInTheDocument();
      });
    });

    it('does not import when confirmation is rejected', async () => {
      const user = userEvent.setup();
      mockConfirm.mockResolvedValue(false);

      renderWithToaster(<Backup />);

      await user.click(
        screen.getByRole('button', { name: 'データのインポート' })
      );

      await waitFor(() => {
        expect(mockConfirm).toHaveBeenCalled();
      });

      expect(mockOpen).not.toHaveBeenCalled();
      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it('does not import when dialog is cancelled', async () => {
      const user = userEvent.setup();
      mockConfirm.mockResolvedValue(true);
      mockOpen.mockResolvedValue(null);

      renderWithToaster(<Backup />);

      await user.click(
        screen.getByRole('button', { name: 'データのインポート' })
      );

      await waitFor(() => {
        expect(mockOpen).toHaveBeenCalled();
      });

      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it('shows error toast when import fails', async () => {
      const user = userEvent.setup();
      mockConfirm.mockResolvedValue(true);
      mockOpen.mockResolvedValue('/path/to/import.zip');
      mockInvoke.mockRejectedValue(new Error('Import failed'));

      renderWithToaster(<Backup />);

      await user.click(
        screen.getByRole('button', { name: 'データのインポート' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(/インポートに失敗しました/)
        ).toBeInTheDocument();
      });
    });
  });

  describe('handleRestore', () => {
    it('restores from restore point without file picker', async () => {
      const user = userEvent.setup();
      mockConfirm.mockResolvedValue(true);
      mockInvoke.mockResolvedValue({
        images_inserted: 1,
        shop_settings_inserted: 1,
        product_master_inserted: 1,
        emails_inserted: 1,
        item_overrides_inserted: 0,
        order_overrides_inserted: 0,
        excluded_items_inserted: 0,
        excluded_orders_inserted: 0,
        image_files_copied: 1,
      });

      renderWithToaster(<Backup />);

      await user.click(
        screen.getByRole('button', { name: '復元（復元ポイント）' })
      );

      await waitFor(() => {
        expect(mockConfirm).toHaveBeenCalled();
      });

      expect(mockOpen).not.toHaveBeenCalled();

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('restore_metadata');
      });

      await waitFor(() => {
        expect(
          screen.getByText(/復元しました（復元ポイント）/)
        ).toBeInTheDocument();
      });
    });

    it('does not restore when confirmation is rejected', async () => {
      const user = userEvent.setup();
      mockConfirm.mockResolvedValue(false);

      renderWithToaster(<Backup />);

      await user.click(
        screen.getByRole('button', { name: '復元（復元ポイント）' })
      );

      await waitFor(() => {
        expect(mockConfirm).toHaveBeenCalled();
      });

      expect(mockInvoke).not.toHaveBeenCalled();
    });
  });

  describe('button disabled states', () => {
    it('disables all buttons when export is in progress', async () => {
      const user = userEvent.setup();

      // Use deferred promises for deterministic control
      let resolveSave: (value: string) => void;
      const savePromise = new Promise<string>((resolve) => {
        resolveSave = resolve;
      });
      mockSave.mockReturnValue(savePromise);

      let resolveInvoke: (value: unknown) => void;
      const invokePromise = new Promise((resolve) => {
        resolveInvoke = resolve;
      });
      mockInvoke.mockReturnValue(invokePromise);

      renderWithToaster(<Backup />);

      const exportButton = screen.getByRole('button', {
        name: 'データのバックアップ',
      });
      const importButton = screen.getByRole('button', {
        name: 'データのインポート',
      });
      const restoreButton = screen.getByRole('button', {
        name: '復元（復元ポイント）',
      });

      // Initially all buttons should be enabled
      expect(exportButton).not.toBeDisabled();
      expect(importButton).not.toBeDisabled();
      expect(restoreButton).not.toBeDisabled();

      // Start export
      await user.click(exportButton);

      // Wait for save dialog to be called
      await waitFor(() => {
        expect(mockSave).toHaveBeenCalled();
      });

      // Resolve save dialog
      resolveSave!('/path/to/export.zip');

      // All buttons should be disabled while export is in progress
      await waitFor(() => {
        expect(exportButton).toBeDisabled();
        expect(importButton).toBeDisabled();
        expect(restoreButton).toBeDisabled();
      });

      // Complete the export
      resolveInvoke!({
        images_count: 10,
        shop_settings_count: 2,
        product_master_count: 50,
        emails_count: 30,
        item_overrides_count: 5,
        order_overrides_count: 3,
        excluded_items_count: 1,
        excluded_orders_count: 2,
        image_files_count: 45,
        images_skipped: 0,
      });

      // After export completes, all buttons should be enabled again
      await waitFor(() => {
        expect(exportButton).not.toBeDisabled();
      });
      expect(importButton).not.toBeDisabled();
      expect(restoreButton).not.toBeDisabled();
    });

    it('disables all buttons when import is in progress', async () => {
      const user = userEvent.setup();
      mockConfirm.mockResolvedValue(true);

      // Use deferred promises for deterministic control
      let resolveOpen: (value: string) => void;
      const openPromise = new Promise<string>((resolve) => {
        resolveOpen = resolve;
      });
      mockOpen.mockReturnValue(openPromise);

      let resolveInvoke: (value: unknown) => void;
      const invokePromise = new Promise((resolve) => {
        resolveInvoke = resolve;
      });
      mockInvoke.mockReturnValue(invokePromise);

      renderWithToaster(<Backup />);

      const exportButton = screen.getByRole('button', {
        name: 'データのバックアップ',
      });
      const importButton = screen.getByRole('button', {
        name: 'データのインポート',
      });
      const restoreButton = screen.getByRole('button', {
        name: '復元（復元ポイント）',
      });

      // Start import
      await user.click(importButton);

      // Wait for confirm to be called
      await waitFor(() => {
        expect(mockConfirm).toHaveBeenCalled();
      });

      // Wait for file picker to be called
      await waitFor(() => {
        expect(mockOpen).toHaveBeenCalled();
      });

      // Resolve file picker
      resolveOpen!('/path/to/import.zip');

      // All buttons should be disabled while import is in progress
      await waitFor(() => {
        expect(exportButton).toBeDisabled();
        expect(importButton).toBeDisabled();
        expect(restoreButton).toBeDisabled();
      });

      // Complete the import
      resolveInvoke!({
        images_inserted: 8,
        shop_settings_inserted: 1,
        product_master_inserted: 40,
        emails_inserted: 25,
        item_overrides_inserted: 4,
        order_overrides_inserted: 2,
        excluded_items_inserted: 1,
        excluded_orders_inserted: 1,
        image_files_copied: 38,
      });

      // After import completes, all buttons should be enabled again
      await waitFor(() => {
        expect(importButton).not.toBeDisabled();
      });
      expect(exportButton).not.toBeDisabled();
      expect(restoreButton).not.toBeDisabled();
    });

    it('disables all buttons when restore is in progress', async () => {
      const user = userEvent.setup();
      mockConfirm.mockResolvedValue(true);

      // Use deferred promises for deterministic control
      let resolveInvoke: (value: unknown) => void;
      const invokePromise = new Promise((resolve) => {
        resolveInvoke = resolve;
      });
      mockInvoke.mockReturnValue(invokePromise);

      renderWithToaster(<Backup />);

      const exportButton = screen.getByRole('button', {
        name: 'データのバックアップ',
      });
      const importButton = screen.getByRole('button', {
        name: 'データのインポート',
      });
      const restoreButton = screen.getByRole('button', {
        name: '復元（復元ポイント）',
      });

      // Start restore
      await user.click(restoreButton);

      // Wait for confirm to be called
      await waitFor(() => {
        expect(mockConfirm).toHaveBeenCalled();
      });

      // All buttons should be disabled while restore is in progress
      await waitFor(() => {
        expect(exportButton).toBeDisabled();
        expect(importButton).toBeDisabled();
        expect(restoreButton).toBeDisabled();
      });

      // Complete the restore
      resolveInvoke!({
        images_inserted: 1,
        shop_settings_inserted: 1,
        product_master_inserted: 1,
        emails_inserted: 1,
        item_overrides_inserted: 0,
        order_overrides_inserted: 0,
        excluded_items_inserted: 0,
        excluded_orders_inserted: 0,
        image_files_copied: 1,
      });

      // After restore completes, all buttons should be enabled again
      await waitFor(() => {
        expect(restoreButton).not.toBeDisabled();
      });
      expect(exportButton).not.toBeDisabled();
      expect(importButton).not.toBeDisabled();
    });
  });
});
