/**
 * UI 一括パースパイプラインの Context 型定義
 *
 * バックエンドの `start_full_parse_pipeline` コマンドを呼び出し、
 * ① メールパース → ② 駿河屋HTMLパース → ③ 商品名解析 → ④ 配送状況確認
 * を順番に実行する。
 */

import { createContext } from 'react';

/** バックエンドの `PipelineStep` enum に対応する型 */
export type PipelineStep = 'parse' | 'product_parse' | 'delivery_check';

/** 各ステップの日本語ラベル */
export const PIPELINE_STEP_LABELS: Record<PipelineStep, string> = {
  parse: 'メールパース（HTML含む）',
  product_parse: '商品名解析',
  delivery_check: '配送状況確認',
};

export interface FullParsePipelineContextValue {
  /** パイプラインが実行中かどうか */
  isRunning: boolean;
  /** 現在実行中のステップ（未実行時は null） */
  currentStep: PipelineStep | null;
  /** 一括パースパイプラインを開始する */
  startPipeline: () => Promise<void>;
}

export const FullParsePipelineContext =
  createContext<FullParsePipelineContextValue | null>(null);
