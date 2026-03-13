import { useContext } from 'react';
import {
  FullParsePipelineContext,
  type FullParsePipelineContextValue,
} from './full-parse-pipeline-context-value';

export function useFullParsePipeline(): FullParsePipelineContextValue {
  const ctx = useContext(FullParsePipelineContext);
  if (!ctx) {
    throw new Error(
      'useFullParsePipeline must be used within FullParsePipelineProvider'
    );
  }
  return ctx;
}
