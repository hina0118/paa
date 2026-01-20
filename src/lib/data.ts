import { Email } from './types';

export const emailData: Email[] = [
  {
    id: '1',
    from: '山田太郎 <yamada@example.com>',
    subject: 'プロジェクトの進捗について',
    preview:
      'お疲れ様です。先日お話しした新規プロジェクトの件ですが、順調に進んでおります。来週のミーティングで詳細を...',
    date: new Date('2024-01-10T09:30:00'),
    read: false,
    starred: true,
    labels: ['work', 'important'],
  },
  {
    id: '2',
    from: '鈴木花子 <suzuki@example.com>',
    subject: '明日の会議の件',
    preview:
      '明日の会議ですが、時間が1時間繰り上がりまして、14時からとなります。ご確認お願いします...',
    date: new Date('2024-01-10T08:15:00'),
    read: true,
    starred: false,
    labels: ['work'],
  },
  {
    id: '3',
    from: '佐藤健 <sato@example.com>',
    subject: 'Re: 見積書の件',
    preview:
      '見積書を確認いたしました。いくつか質問がありますので、お時間のある時にお電話いただけますでしょうか...',
    date: new Date('2024-01-09T16:45:00'),
    read: true,
    starred: false,
    labels: ['work', 'finance'],
  },
  {
    id: '4',
    from: '田中美咲 <tanaka@example.com>',
    subject: '週末のイベントについて',
    preview:
      'こんにちは!週末のイベントの参加者リストを送付します。全員で15名の参加予定となっております...',
    date: new Date('2024-01-09T14:20:00'),
    read: false,
    starred: false,
    labels: ['personal'],
  },
  {
    id: '5',
    from: 'システム管理者 <admin@example.com>',
    subject: '【重要】システムメンテナンスのお知らせ',
    preview:
      'システムメンテナンスを以下の日程で実施いたします。メンテナンス中はサービスをご利用いただけません...',
    date: new Date('2024-01-09T10:00:00'),
    read: false,
    starred: true,
    labels: ['system', 'important'],
  },
  {
    id: '6',
    from: '高橋誠 <takahashi@example.com>',
    subject: '資料の共有',
    preview:
      'お疲れ様です。先日のプレゼンテーション資料を共有いたします。ご確認の上、フィードバックをいただけると...',
    date: new Date('2024-01-08T17:30:00'),
    read: true,
    starred: false,
    labels: ['work'],
  },
  {
    id: '7',
    from: '伊藤直美 <ito@example.com>',
    subject: '新年会の出欠確認',
    preview:
      '新年会の日程が決まりましたので、出欠のご確認をお願いいたします。1月20日(土)18時から...',
    date: new Date('2024-01-08T12:00:00'),
    read: true,
    starred: false,
    labels: ['personal'],
  },
  {
    id: '8',
    from: '渡辺真一 <watanabe@example.com>',
    subject: '契約書のドラフト',
    preview:
      '契約書のドラフトを作成しましたので、ご確認をお願いいたします。修正点がございましたら...',
    date: new Date('2024-01-07T15:10:00'),
    read: false,
    starred: true,
    labels: ['work', 'legal'],
  },
  {
    id: '9',
    from: '中村雅子 <nakamura@example.com>',
    subject: '研修資料について',
    preview:
      '来月の研修で使用する資料の準備状況を確認させていただきたく、ご連絡いたしました...',
    date: new Date('2024-01-07T11:25:00'),
    read: true,
    starred: false,
    labels: ['work', 'training'],
  },
  {
    id: '10',
    from: '小林優子 <kobayashi@example.com>',
    subject: 'お礼とご報告',
    preview:
      '先日はお忙しい中、お時間をいただきありがとうございました。ご相談させていただいた件につきまして...',
    date: new Date('2024-01-06T13:40:00'),
    read: true,
    starred: false,
    labels: ['personal'],
  },
];
