/**
 * ニュースソースのレジストリ
 *
 * 新しいソースを追加する手順:
 *   1. `./sourcename.ts` を作成し NewsSource を export
 *   2. ここに import して allNewsSources に追加
 *   RSS 非対応サイトは htmlSelectors を設定することでスクレイピングで取得可能
 */
import type { NewsSource } from '../types';
import { denfamiSource } from './denfami';
import { fourGamerSource } from './4gamer';
import { famitsuSource } from './famitsu';
import { gamesparkSource } from './gamespark';
import { insideSource } from './inside';
import { automatonSource } from './automaton';
import { gamerSource } from './gamer';
import { dengekiHobbySource } from './dengeki-hobby';
import { hobbyWatchSource } from './hobby-watch';
import { hobbyManiaxSource } from './hobby-maniax';
import { hjwebSource } from './hjweb';

export {
  denfamiSource,
  fourGamerSource,
  famitsuSource,
  gamesparkSource,
  insideSource,
  automatonSource,
  gamerSource,
  dengekiHobbySource,
  hobbyWatchSource,
  hobbyManiaxSource,
  hjwebSource,
};

export const allNewsSources: NewsSource[] = [
  denfamiSource,
  fourGamerSource,
  famitsuSource,
  gamesparkSource,
  insideSource,
  automatonSource,
  gamerSource,
  dengekiHobbySource,
  hobbyWatchSource,
  hobbyManiaxSource,
  hjwebSource,
];
