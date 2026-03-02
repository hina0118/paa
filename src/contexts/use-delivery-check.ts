import { useContext } from 'react';
import { DeliveryCheckContext } from './delivery-check-context-value';

export function useDeliveryCheck() {
  const ctx = useContext(DeliveryCheckContext);
  if (!ctx) {
    throw new Error(
      'useDeliveryCheck must be used within a DeliveryCheckProvider'
    );
  }
  return ctx;
}
