import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Toaster } from 'sonner';
import { ImageSearchDialog } from './image-search-dialog';
import { mockInvoke } from '@/test/setup';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

vi.mock('@tauri-apps/api/webviewWindow', () => ({
  WebviewWindow: vi.fn().mockImplementation(() => ({
    once: vi.fn(),
  })),
}));

const renderWithToaster = (ui: React.ReactElement) =>
  render(
    <>
      {ui}
      <Toaster position="top-right" richColors />
    </>
  );

describe('ImageSearchDialog', () => {
  const defaultProps = {
    open: true,
    onOpenChange: vi.fn(),
    itemId: 1,
    itemName: 'テスト商品',
    onImageSaved: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(undefined);
    vi.mocked(WebviewWindow).mockImplementation(() => ({
      once: vi.fn(),
    }));
  });

  it('renders dialog when open', () => {
    render(<ImageSearchDialog {...defaultProps} />);
    expect(
      screen.getByRole('heading', { name: '画像を検索' })
    ).toBeInTheDocument();
    expect(screen.getByText(/テスト商品/)).toBeInTheDocument();
  });

  it('calls search_product_images when search button is clicked', async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue([]);

    renderWithToaster(<ImageSearchDialog {...defaultProps} />);

    const dialog = screen.getByRole('dialog');
    await user.click(
      within(dialog).getByRole('button', { name: /画像を検索/ })
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('search_product_images', {
        query: 'テスト商品',
        numResults: 10,
      });
    });
  });

  it('shows error when search returns no results', async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue([]);

    renderWithToaster(<ImageSearchDialog {...defaultProps} />);

    const dialog = screen.getByRole('dialog');
    await user.click(
      within(dialog).getByRole('button', { name: /画像を検索/ })
    );

    await waitFor(() => {
      expect(
        screen.getByText('画像が見つかりませんでした。')
      ).toBeInTheDocument();
    });
  });

  it('shows error when search fails', async () => {
    const user = userEvent.setup();
    mockInvoke.mockRejectedValue(new Error('Network error'));

    renderWithToaster(<ImageSearchDialog {...defaultProps} />);

    const dialog = screen.getByRole('dialog');
    await user.click(
      within(dialog).getByRole('button', { name: /画像を検索/ })
    );

    await waitFor(() => {
      expect(screen.getByText(/Network error/)).toBeInTheDocument();
    });
  });

  it('displays search results and allows selection', async () => {
    const user = userEvent.setup();
    const mockResults = [
      {
        url: 'https://example.com/img1.jpg',
        thumbnail_url: 'https://example.com/thumb1.jpg',
        width: 100,
        height: 100,
        title: '画像1',
        mime_type: 'image/jpeg',
      },
      {
        url: 'https://example.com/img2.jpg',
        thumbnail_url: null,
        width: null,
        height: null,
        title: null,
        mime_type: null,
      },
    ];
    mockInvoke.mockResolvedValue(mockResults);

    renderWithToaster(<ImageSearchDialog {...defaultProps} />);

    const dialog = screen.getByRole('dialog');
    await user.click(
      within(dialog).getByRole('button', { name: /画像を検索/ })
    );

    await waitFor(() => {
      expect(screen.getByAltText('画像1')).toBeInTheDocument();
      expect(screen.getByAltText('検索結果 2')).toBeInTheDocument();
    });

    const img1 = screen.getByAltText('画像1');
    const firstResultButton = img1.closest('button');
    if (firstResultButton) {
      await user.click(firstResultButton);
    }

    await waitFor(() => {
      expect(screen.getByText(/選択中の画像/)).toBeInTheDocument();
    });
  });

  it('saves selected image when save button is clicked', async () => {
    const user = userEvent.setup();
    const mockResults = [
      {
        url: 'https://example.com/img1.jpg',
        thumbnail_url: null,
        width: null,
        height: null,
        title: null,
        mime_type: null,
      },
    ];
    mockInvoke
      .mockResolvedValueOnce(mockResults)
      .mockResolvedValueOnce(undefined);

    renderWithToaster(<ImageSearchDialog {...defaultProps} />);

    const dialog = screen.getByRole('dialog');
    await user.click(
      within(dialog).getByRole('button', { name: /画像を検索/ })
    );

    await waitFor(() => {
      expect(screen.getByAltText('検索結果 1')).toBeInTheDocument();
    });

    const img = screen.getByAltText('検索結果 1');
    const firstResultButton = img.closest('button');
    if (firstResultButton) {
      await user.click(firstResultButton);
    }

    await waitFor(() => {
      expect(
        screen.getByRole('button', { name: '選択した画像を保存' })
      ).toBeInTheDocument();
    });

    await user.click(
      screen.getByRole('button', { name: '選択した画像を保存' })
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('save_image_from_url', {
        itemId: 1,
        imageUrl: 'https://example.com/img1.jpg',
      });
    });

    await waitFor(() => {
      expect(screen.getByText('画像を保存しました')).toBeInTheDocument();
    });

    expect(defaultProps.onImageSaved).toHaveBeenCalled();
  });

  it('shows error when save fails', async () => {
    const user = userEvent.setup();
    const mockResults = [
      {
        url: 'https://example.com/img1.jpg',
        thumbnail_url: null,
        width: null,
        height: null,
        title: null,
        mime_type: null,
      },
    ];
    mockInvoke
      .mockResolvedValueOnce(mockResults)
      .mockRejectedValueOnce(new Error('Save failed'));

    renderWithToaster(<ImageSearchDialog {...defaultProps} />);

    const dialog = screen.getByRole('dialog');
    await user.click(
      within(dialog).getByRole('button', { name: /画像を検索/ })
    );

    await waitFor(() => {
      expect(screen.getByAltText('検索結果 1')).toBeInTheDocument();
    });

    const img = screen.getByAltText('検索結果 1');
    const firstResultButton = img.closest('button');
    if (firstResultButton) {
      await user.click(firstResultButton);
    }

    await user.click(
      screen.getByRole('button', { name: '選択した画像を保存' })
    );

    await waitFor(() => {
      expect(screen.getByText(/Save failed/)).toBeInTheDocument();
    });
  });

  it('does not save when no image is selected', async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue([]);

    renderWithToaster(<ImageSearchDialog {...defaultProps} />);

    const dialog = screen.getByRole('dialog');
    await user.click(
      within(dialog).getByRole('button', { name: /画像を検索/ })
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'search_product_images',
        expect.any(Object)
      );
    });

    const saveButton = screen.getByRole('button', {
      name: '選択した画像を保存',
    });
    expect(saveButton).toBeDisabled();
  });

  it('calls onOpenChange when cancel is clicked', async () => {
    const user = userEvent.setup();
    render(<ImageSearchDialog {...defaultProps} />);

    await user.click(screen.getByRole('button', { name: 'キャンセル' }));

    expect(defaultProps.onOpenChange).toHaveBeenCalledWith(false);
  });

  it('handles non-Error in catch block', async () => {
    const user = userEvent.setup();
    mockInvoke.mockRejectedValue('string error');

    renderWithToaster(<ImageSearchDialog {...defaultProps} />);

    const dialog = screen.getByRole('dialog');
    await user.click(
      within(dialog).getByRole('button', { name: /画像を検索/ })
    );

    await waitFor(() => {
      expect(screen.getByText(/string error/)).toBeInTheDocument();
    });
  });

  it('shows fallback message and opens sub-window when API search fails', async () => {
    const user = userEvent.setup();
    mockInvoke.mockRejectedValue(new Error('API limit reached'));

    renderWithToaster(<ImageSearchDialog {...defaultProps} />);

    const dialog = screen.getByRole('dialog');
    await user.click(
      within(dialog).getByRole('button', { name: /画像を検索/ })
    );

    await waitFor(() => {
      expect(
        screen.getByText(
          /結果が見つからない場合もサブウィンドウでGoogle画像検索を開き/
        )
      ).toBeInTheDocument();
    });

    const browserButton = screen.getByRole('button', {
      name: /サブウィンドウでGoogle画像検索を開く/,
    });
    await user.click(browserButton);

    expect(WebviewWindow).toHaveBeenCalledWith(
      expect.stringMatching(/^image-search-.+$/),
      expect.objectContaining({
        url: 'https://www.google.com/search?q=%E3%83%86%E3%82%B9%E3%83%88%E5%95%86%E5%93%81&tbm=isch',
        title: 'Google画像検索: テスト商品',
        width: 900,
        height: 700,
      })
    );
  });

  it('saves image from manual URL input', async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue(undefined);

    renderWithToaster(<ImageSearchDialog {...defaultProps} />);

    const urlInput = screen.getByPlaceholderText('画像のURLをここに貼り付け');
    await user.type(urlInput, 'https://example.com/my-image.jpg');

    const saveButton = screen.getByRole('button', {
      name: '選択した画像を保存',
    });
    expect(saveButton).not.toBeDisabled();
    await user.click(saveButton);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('save_image_from_url', {
        itemId: 1,
        imageUrl: 'https://example.com/my-image.jpg',
      });
    });
  });
});
