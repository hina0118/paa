import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Store } from 'lucide-react';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';

interface ShopSetting {
  id: number;
  shop_name: string;
  sender_address: string;
  parser_type: string;
  is_enabled: boolean;
  subject_filters: string | null; // JSON array stored as string
  created_at: string;
  updated_at: string;
}

interface ShopSettingDisplay extends Omit<ShopSetting, 'subject_filters'> {
  subject_filters_array: string[]; // Parsed array for display
}

export function ShopSettings() {
  const [shops, setShops] = useState<ShopSettingDisplay[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>('');
  const [successMessage, setSuccessMessage] = useState<string>('');

  // New shop form state
  const [newShopName, setNewShopName] = useState('');
  const [newSenderAddress, setNewSenderAddress] = useState('');
  const [newParserType, setNewParserType] = useState('');
  const [newSubjectFilters, setNewSubjectFilters] = useState<string[]>(['']); // Array of subject filters
  const [isAdding, setIsAdding] = useState(false);

  // Edit state
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editForm, setEditForm] = useState<Partial<ShopSettingDisplay>>({});

  useEffect(() => {
    loadShops();
  }, []);

  const loadShops = async () => {
    try {
      setIsLoading(true);
      setError('');
      const result = await invoke<ShopSetting[]>('get_all_shop_settings');
      // Parse JSON subject_filters to array for display
      const displayShops: ShopSettingDisplay[] = result.map((shop) => ({
        ...shop,
        subject_filters_array: shop.subject_filters
          ? JSON.parse(shop.subject_filters)
          : [],
      }));
      setShops(displayShops);
    } catch (err) {
      setError(
        `読み込みに失敗しました: ${err instanceof Error ? err.message : String(err)}`
      );
    } finally {
      setIsLoading(false);
    }
  };

  const handleAdd = async () => {
    if (
      !newShopName.trim() ||
      !newSenderAddress.trim() ||
      !newParserType.trim()
    ) {
      setError('すべての項目を入力してください');
      return;
    }

    // Email validation
    if (!newSenderAddress.includes('@')) {
      setError('有効なメールアドレスを入力してください');
      return;
    }

    try {
      setIsAdding(true);
      setError('');

      // Filter out empty strings from subject filters
      const cleanedFilters = newSubjectFilters
        .map((f) => f.trim())
        .filter((f) => f.length > 0);

      await invoke('create_shop_setting', {
        shopName: newShopName,
        senderAddress: newSenderAddress.toLowerCase(),
        parserType: newParserType,
        subjectFilters: cleanedFilters.length > 0 ? cleanedFilters : null,
      });
      setSuccessMessage('新しい店舗設定を追加しました');
      setTimeout(() => setSuccessMessage(''), 3000);
      setNewShopName('');
      setNewSenderAddress('');
      setNewParserType('');
      setNewSubjectFilters(['']);
      await loadShops();
    } catch (err) {
      setError(
        `追加に失敗しました: ${err instanceof Error ? err.message : String(err)}`
      );
    } finally {
      setIsAdding(false);
    }
  };

  const handleEdit = (shop: ShopSettingDisplay) => {
    setEditingId(shop.id);
    setEditForm({
      shop_name: shop.shop_name,
      sender_address: shop.sender_address,
      parser_type: shop.parser_type,
      is_enabled: shop.is_enabled,
      subject_filters_array:
        shop.subject_filters_array.length > 0
          ? shop.subject_filters_array
          : [''],
    });
  };

  const handleCancelEdit = () => {
    setEditingId(null);
    setEditForm({});
  };

  const handleSaveEdit = async (id: number) => {
    if (
      !editForm.shop_name?.trim() ||
      !editForm.sender_address?.trim() ||
      !editForm.parser_type?.trim()
    ) {
      setError('すべての項目を入力してください');
      return;
    }

    if (!editForm.sender_address.includes('@')) {
      setError('有効なメールアドレスを入力してください');
      return;
    }

    try {
      setError('');

      // Filter out empty strings from subject filters
      const cleanedFilters = (editForm.subject_filters_array || [])
        .map((f) => f.trim())
        .filter((f) => f.length > 0);

      await invoke('update_shop_setting', {
        id,
        shopName: editForm.shop_name,
        senderAddress: editForm.sender_address?.toLowerCase(),
        parserType: editForm.parser_type,
        isEnabled: editForm.is_enabled,
        subjectFilters: cleanedFilters.length > 0 ? cleanedFilters : null,
      });
      setSuccessMessage('店舗設定を更新しました');
      setTimeout(() => setSuccessMessage(''), 3000);
      setEditingId(null);
      setEditForm({});
      await loadShops();
    } catch (err) {
      setError(
        `更新に失敗しました: ${err instanceof Error ? err.message : String(err)}`
      );
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm('この店舗設定を削除してもよろしいですか?')) {
      return;
    }

    try {
      setError('');
      await invoke('delete_shop_setting', { id });
      setSuccessMessage('店舗設定を削除しました');
      setTimeout(() => setSuccessMessage(''), 3000);
      await loadShops();
    } catch (err) {
      setError(
        `削除に失敗しました: ${err instanceof Error ? err.message : String(err)}`
      );
    }
  };

  const handleToggleEnabled = async (shop: ShopSettingDisplay) => {
    try {
      setError('');
      await invoke('update_shop_setting', {
        id: shop.id,
        shopName: null,
        senderAddress: null,
        parserType: null,
        isEnabled: !shop.is_enabled,
        subjectFilters: null,
      });
      await loadShops();
    } catch (err) {
      setError(
        `更新に失敗しました: ${err instanceof Error ? err.message : String(err)}`
      );
    }
  };

  return (
    <div className="container mx-auto py-10 px-6 space-y-6">
      <div className="mb-8 space-y-2">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-primary/10">
            <Store className="h-6 w-6 text-primary" />
          </div>
          <h1 className="text-3xl font-bold tracking-tight">店舗設定</h1>
        </div>
      </div>

      {successMessage && (
        <div
          className="p-3 bg-green-50 border border-green-200 rounded text-sm text-green-800"
          data-testid="success-message"
          role="status"
        >
          {successMessage}
        </div>
      )}

      {error && (
        <div
          className="p-3 bg-red-50 border border-red-200 rounded text-sm text-red-800"
          data-testid="error-message"
          role="alert"
        >
          {error}
        </div>
      )}

      {/* Add new shop */}
      <Card>
        <CardHeader>
          <CardTitle>新しい店舗を追加</CardTitle>
          <CardDescription>
            取り込み対象のメール送信元アドレスと解析ロジックを設定します
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4">
            <div className="grid grid-cols-3 gap-4">
              <div className="space-y-2">
                <label htmlFor="shop-name" className="text-sm font-medium">
                  店舗名
                </label>
                <Input
                  id="shop-name"
                  placeholder="例: Amazon発送通知"
                  value={newShopName}
                  onChange={(e) => setNewShopName(e.target.value)}
                  disabled={isAdding}
                />
              </div>
              <div className="space-y-2">
                <label htmlFor="sender-address" className="text-sm font-medium">
                  送信元アドレス
                </label>
                <Input
                  id="sender-address"
                  type="email"
                  placeholder="例: ship-confirm@amazon.co.jp"
                  value={newSenderAddress}
                  onChange={(e) => setNewSenderAddress(e.target.value)}
                  disabled={isAdding}
                />
              </div>
              <div className="space-y-2">
                <label htmlFor="parser-type" className="text-sm font-medium">
                  パーサータイプ
                </label>
                <Input
                  id="parser-type"
                  placeholder="例: amazon"
                  value={newParserType}
                  onChange={(e) => setNewParserType(e.target.value)}
                  disabled={isAdding}
                />
              </div>
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">
                件名フィルター（オプション）
              </label>
              <div className="space-y-2">
                {newSubjectFilters.map((filter, index) => (
                  <div key={index} className="flex gap-2">
                    <Input
                      placeholder="例: 【ホビーサーチ】ご注文の発送が完了しました"
                      value={filter}
                      onChange={(e) => {
                        const updated = [...newSubjectFilters];
                        updated[index] = e.target.value;
                        setNewSubjectFilters(updated);
                      }}
                      disabled={isAdding}
                    />
                    {newSubjectFilters.length > 1 && (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          const updated = newSubjectFilters.filter(
                            (_, i) => i !== index
                          );
                          setNewSubjectFilters(updated);
                        }}
                        disabled={isAdding}
                      >
                        削除
                      </Button>
                    )}
                  </div>
                ))}
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    setNewSubjectFilters([...newSubjectFilters, ''])
                  }
                  disabled={isAdding}
                >
                  + 件名パターンを追加
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                設定した場合、いずれかの件名パターンを含むメールのみを取り込みます
              </p>
            </div>
            <div>
              <Button onClick={handleAdd} disabled={isAdding}>
                {isAdding ? '追加中...' : '追加'}
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Shop list */}
      <Card>
        <CardHeader>
          <CardTitle>登録済み店舗</CardTitle>
          <CardDescription>
            {shops.length}件の店舗が登録されています
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <p className="text-sm text-muted-foreground">読み込み中...</p>
          ) : shops.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              店舗が登録されていません
            </p>
          ) : (
            <div className="space-y-4">
              {shops.map((shop) => (
                <div key={shop.id} className="p-4 border rounded-lg space-y-3">
                  {editingId === shop.id ? (
                    // Edit mode
                    <div className="space-y-3">
                      <div className="grid grid-cols-3 gap-4">
                        <div className="space-y-2">
                          <label className="text-sm font-medium">店舗名</label>
                          <Input
                            value={editForm.shop_name || ''}
                            onChange={(e) =>
                              setEditForm({
                                ...editForm,
                                shop_name: e.target.value,
                              })
                            }
                          />
                        </div>
                        <div className="space-y-2">
                          <label className="text-sm font-medium">
                            送信元アドレス
                          </label>
                          <Input
                            type="email"
                            value={editForm.sender_address || ''}
                            onChange={(e) =>
                              setEditForm({
                                ...editForm,
                                sender_address: e.target.value,
                              })
                            }
                          />
                        </div>
                        <div className="space-y-2">
                          <label className="text-sm font-medium">
                            パーサータイプ
                          </label>
                          <Input
                            value={editForm.parser_type || ''}
                            onChange={(e) =>
                              setEditForm({
                                ...editForm,
                                parser_type: e.target.value,
                              })
                            }
                          />
                        </div>
                      </div>
                      <div className="space-y-2">
                        <label className="text-sm font-medium">
                          件名フィルター（オプション）
                        </label>
                        <div className="space-y-2">
                          {(editForm.subject_filters_array || ['']).map(
                            (filter, index) => (
                              <div key={index} className="flex gap-2">
                                <Input
                                  placeholder="例: 【ホビーサーチ】ご注文の発送が完了しました"
                                  value={filter}
                                  onChange={(e) => {
                                    const updated = [
                                      ...(editForm.subject_filters_array || [
                                        '',
                                      ]),
                                    ];
                                    updated[index] = e.target.value;
                                    setEditForm({
                                      ...editForm,
                                      subject_filters_array: updated,
                                    });
                                  }}
                                />
                                {(editForm.subject_filters_array || [''])
                                  .length > 1 && (
                                  <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={() => {
                                      const updated = (
                                        editForm.subject_filters_array || ['']
                                      ).filter((_, i) => i !== index);
                                      setEditForm({
                                        ...editForm,
                                        subject_filters_array: updated,
                                      });
                                    }}
                                  >
                                    削除
                                  </Button>
                                )}
                              </div>
                            )
                          )}
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => {
                              setEditForm({
                                ...editForm,
                                subject_filters_array: [
                                  ...(editForm.subject_filters_array || ['']),
                                  '',
                                ],
                              });
                            }}
                          >
                            + 件名パターンを追加
                          </Button>
                        </div>
                        <p className="text-xs text-muted-foreground">
                          設定した場合、いずれかの件名パターンを含むメールのみを取り込みます
                        </p>
                      </div>
                      <div className="flex items-center gap-2">
                        <Checkbox
                          id={`enabled-${shop.id}`}
                          checked={editForm.is_enabled ?? false}
                          onCheckedChange={(checked) =>
                            setEditForm({
                              ...editForm,
                              is_enabled: checked as boolean,
                            })
                          }
                        />
                        <label
                          htmlFor={`enabled-${shop.id}`}
                          className="text-sm font-medium"
                        >
                          有効
                        </label>
                      </div>
                      <div className="flex gap-2">
                        <Button onClick={() => handleSaveEdit(shop.id)}>
                          保存
                        </Button>
                        <Button variant="outline" onClick={handleCancelEdit}>
                          キャンセル
                        </Button>
                      </div>
                    </div>
                  ) : (
                    // View mode
                    <div className="space-y-2">
                      <div className="flex items-start justify-between">
                        <div className="space-y-1">
                          <div className="flex items-center gap-2">
                            <h3 className="font-semibold">{shop.shop_name}</h3>
                            <span
                              className={`text-xs px-2 py-1 rounded ${
                                shop.is_enabled
                                  ? 'bg-green-100 text-green-800'
                                  : 'bg-gray-100 text-gray-800'
                              }`}
                            >
                              {shop.is_enabled ? '有効' : '無効'}
                            </span>
                          </div>
                          <p className="text-sm text-muted-foreground">
                            {shop.sender_address}
                          </p>
                          <p className="text-sm text-muted-foreground">
                            パーサー: {shop.parser_type}
                          </p>
                          {shop.subject_filters_array &&
                            shop.subject_filters_array.length > 0 && (
                              <div className="text-sm text-muted-foreground">
                                <div className="font-medium">
                                  件名フィルター:
                                </div>
                                <ul className="list-disc list-inside pl-2">
                                  {shop.subject_filters_array.map(
                                    (filter, index) => (
                                      <li key={index}>{filter}</li>
                                    )
                                  )}
                                </ul>
                              </div>
                            )}
                        </div>
                        <div className="flex gap-2">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleToggleEnabled(shop)}
                          >
                            {shop.is_enabled ? '無効化' : '有効化'}
                          </Button>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleEdit(shop)}
                          >
                            編集
                          </Button>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleDelete(shop.id)}
                          >
                            削除
                          </Button>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
