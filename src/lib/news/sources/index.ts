/**
 * ニュースソースのレジストリ
 *
 * 新しいソースを追加する手順:
 *   1. `./sourcename.ts` を作成し NewsSource を export
 *   2. ここに import して allNewsSources に追加
 */
import type { NewsSource } from '../types';
import { denfamiSource } from './denfami';

export { denfamiSource };

export const allNewsSources: NewsSource[] = [denfamiSource];
