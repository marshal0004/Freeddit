'use client';

import { useState } from 'react';
import { Loader2, ChevronDown } from 'lucide-react';

interface LoadMoreButtonProps {
  onLoadMore: () => Promise<void>;
  hasMore: boolean;
  loading?: boolean;
}

export function LoadMoreButton({ onLoadMore, hasMore, loading }: LoadMoreButtonProps) {
  const [internalLoading, setInternalLoading] = useState(false);
  if (!hasMore) return null;
  return (
    <button
      onClick={async () => {
        setInternalLoading(true);
        try { await onLoadMore(); } finally { setInternalLoading(false); }
      }}
      disabled={internalLoading || loading}
      className="w-full py-3 mt-3 bg-card border border-border rounded-lg text-sm font-semibold hover:bg-accent transition-colors flex items-center justify-center gap-2 disabled:opacity-50"
    >
      {internalLoading || loading ? (
        <>
          <Loader2 className="w-4 h-4 animate-spin" />
          Loading...
        </>
      ) : (
        <>
          <ChevronDown className="w-4 h-4" />
          Load more
        </>
      )}
    </button>
  );
}
