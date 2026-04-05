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
import { gamewithSource } from './gamewith';
import { gamesparkSource } from './gamespark';
import { insideSource } from './inside';
import { automatonSource } from './automaton';
import { gamerSource } from './gamer';

export {
  denfamiSource,
  fourGamerSource,
  famitsuSource,
  gamewithSource,
  gamesparkSource,
  insideSource,
  automatonSource,
  gamerSource,
};

export const allNewsSources: NewsSource[] = [
  denfamiSource,
  fourGamerSource,
  famitsuSource,
  gamewithSource,
  gamesparkSource,
  insideSource,
  automatonSource,
  gamerSource,
];
