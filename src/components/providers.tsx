'use client';

import { useState, ReactNode } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { AuthProvider } from '@/lib/auth-context';

export function Providers({ children }: { children: ReactNode }) {
  const [qc] = useState(() => new QueryClient({
    defaultOptions: {
      queries: {
        refetchOnWindowFocus: false,
        retry: 1,
        staleTime: 30_000,
      },
    },
  }));

  return (
    <QueryClientProvider client={qc}>
      <AuthProvider>{children}</AuthProvider>
    </QueryClientProvider>
  );
}
