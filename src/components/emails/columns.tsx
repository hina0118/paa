import { ColumnDef } from '@tanstack/react-table';
import { Email } from '@/lib/types';
import { Checkbox } from '@/components/ui/checkbox';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  MoreHorizontal,
  Star,
  Mail,
  Archive,
  Trash2,
  Copy,
  Tag,
  Clock,
  FileText,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';

// パースハンドラー関数
async function handleParseEmail(email: Email) {
  try {
    if (!email.bodyPlain) {
      toast.error('メール本文がありません');
      return;
    }

    // 送信元アドレスから店舗ドメインを抽出
    const emailMatch = email.from.match(/<(.+?)>/);
    const senderEmail = emailMatch ? emailMatch[1] : email.from;
    const shopDomain = senderEmail.split('@')[1];

    toast.info('メールをパース中...');

    const orderId = await invoke<number>('parse_and_save_email', {
      parserType: 'hobbysearch',
      emailBody: email.bodyPlain,
      emailId: parseInt(email.id),
      shopDomain,
    });

    toast.success(`注文を保存しました (ID: ${orderId})`);
  } catch (error) {
    console.error('Parse error:', error);
    toast.error(`パースに失敗しました: ${error}`);
  }
}

export const columns: ColumnDef<Email>[] = [
  {
    id: 'select',
    header: ({ table }) => (
      <Checkbox
        checked={
          table.getIsAllPageRowsSelected() ||
          (table.getIsSomePageRowsSelected() && 'indeterminate')
        }
        onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
        aria-label="すべて選択"
      />
    ),
    cell: ({ row }) => (
      <Checkbox
        checked={row.getIsSelected()}
        onCheckedChange={(value) => row.toggleSelected(!!value)}
        aria-label="行を選択"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'starred',
    header: '',
    cell: ({ row }) => {
      const starred = row.getValue('starred') as boolean;
      return (
        <Button
          variant="ghost"
          size="sm"
          className="h-8 w-8 p-0 hover:bg-transparent"
          onClick={(e) => {
            e.stopPropagation();
            // TODO: Implement star toggle functionality
          }}
        >
          <Star
            className={`h-4 w-4 transition-colors ${
              starred
                ? 'fill-yellow-400 text-yellow-400'
                : 'text-muted-foreground hover:text-yellow-400'
            }`}
          />
        </Button>
      );
    },
  },
  {
    accessorKey: 'from',
    header: () => (
      <div className="flex items-center gap-2">
        <Mail className="h-4 w-4" />
        <span>送信者</span>
      </div>
    ),
    cell: ({ row }) => {
      const from = row.getValue('from') as string;
      const read = row.original.read;
      return (
        <div className="flex items-center gap-2">
          <div
            className={`h-2 w-2 rounded-full ${
              !read ? 'bg-blue-500' : 'bg-transparent'
            }`}
          />
          <div className={`font-medium ${!read ? 'font-bold' : ''}`}>
            {from.split('<')[0].trim()}
          </div>
        </div>
      );
    },
  },
  {
    accessorKey: 'subject',
    header: '件名',
    cell: ({ row }) => {
      const subject = row.getValue('subject') as string;
      const preview = row.original.preview;
      const read = row.original.read;
      return (
        <div className="max-w-[500px]">
          <div className={`${!read ? 'font-bold' : ''}`}>{subject}</div>
          <div className="text-sm text-muted-foreground truncate">
            {preview}
          </div>
        </div>
      );
    },
  },
  {
    accessorKey: 'labels',
    header: () => (
      <div className="flex items-center gap-2">
        <Tag className="h-4 w-4" />
        <span>ラベル</span>
      </div>
    ),
    cell: ({ row }) => {
      const labels = row.getValue('labels') as string[];
      return (
        <div className="flex gap-1 flex-wrap">
          {labels.map((label) => (
            <span
              key={label}
              className="inline-flex items-center gap-1 px-2.5 py-0.5 text-xs font-medium rounded-full bg-primary/10 text-primary border border-primary/20"
            >
              <Tag className="h-3 w-3" />
              {label}
            </span>
          ))}
        </div>
      );
    },
  },
  {
    accessorKey: 'date',
    header: () => (
      <div className="flex items-center gap-2">
        <Clock className="h-4 w-4" />
        <span>日時</span>
      </div>
    ),
    cell: ({ row }) => {
      const date = row.getValue('date') as Date;
      return (
        <div className="flex items-center gap-2 text-sm text-muted-foreground whitespace-nowrap">
          <Clock className="h-3.5 w-3.5" />
          {date.toLocaleDateString('ja-JP', {
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
          })}
        </div>
      );
    },
  },
  {
    id: 'actions',
    cell: ({ row }) => {
      const email = row.original;

      return (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" className="h-8 w-8 p-0">
              <span className="sr-only">メニューを開く</span>
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuLabel>アクション</DropdownMenuLabel>
            <DropdownMenuItem
              onClick={() => navigator.clipboard.writeText(email.id)}
            >
              <Copy className="mr-2 h-4 w-4" />
              IDをコピー
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={() => handleParseEmail(email)}>
              <FileText className="mr-2 h-4 w-4" />
              パースして保存
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem>
              <Mail className="mr-2 h-4 w-4" />
              既読にする
            </DropdownMenuItem>
            <DropdownMenuItem>
              <Archive className="mr-2 h-4 w-4" />
              アーカイブ
            </DropdownMenuItem>
            <DropdownMenuItem className="text-destructive">
              <Trash2 className="mr-2 h-4 w-4" />
              削除
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      );
    },
  },
];
