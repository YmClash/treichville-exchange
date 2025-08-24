import { useState, useEffect } from 'react';

export type ToastVariant = 'default' | 'success' | 'destructive' | 'warning';

interface Toast {
  id: string;
  title: string;
  description?: string;
  variant?: ToastVariant;
  duration?: number;
}

interface ToastState {
  toasts: Toast[];
}

let listeners: Array<(state: ToastState) => void> = [];
let memoryState: ToastState = { toasts: [] };

function dispatch(action: any) {
  memoryState = action(memoryState);
  listeners.forEach((listener) => {
    listener(memoryState);
  });
}

export function toast({
  title,
  description,
  variant = 'default',
  duration = 5000,
}: Omit<Toast, 'id'>) {
  const id = Math.random().toString(36).substr(2, 9);
  
  dispatch((state: ToastState) => ({
    ...state,
    toasts: [...state.toasts, { id, title, description, variant, duration }],
  }));

  if (duration > 0) {
    setTimeout(() => {
      dispatch((state: ToastState) => ({
        ...state,
        toasts: state.toasts.filter((t) => t.id !== id),
      }));
    }, duration);
  }
}

export function useToast() {
  const [state, setState] = useState<ToastState>(memoryState);

  useEffect(() => {
    listeners.push(setState);
    return () => {
      const index = listeners.indexOf(setState);
      if (index > -1) {
        listeners.splice(index, 1);
      }
    };
  }, [state]);

  return {
    toasts: state.toasts,
    toast,
    dismiss: (id: string) => {
      dispatch((state: ToastState) => ({
        ...state,
        toasts: state.toasts.filter((t) => t.id !== id),
      }));
    },
    dismissAll: () => {
      dispatch(() => ({ toasts: [] }));
    },
  };
}