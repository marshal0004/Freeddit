'use client';
import { useAuth } from '@/lib/auth-context';

import { useState } from 'react';
import { ChevronUp, ChevronDown } from 'lucide-react';
import { api } from '@/lib/api';
import { cn } from '@/lib/utils';

interface VoteButtonProps {
  targetId: string;
  score: number;
  initialLikes: number | null;
  onVoted?: (newScore: number, newLikes: number) => void;
  variant?: 'post' | 'comment';
}

export function VoteButton({ targetId, score, initialLikes, onVoted, variant = 'post' }: VoteButtonProps) {
  const { user } = useAuth();
  const [likes, setLikes] = useState<number | null>(initialLikes);
  const [currentScore, setCurrentScore] = useState(score);
  const [busy, setBusy] = useState(false);

  const handleVote = async (direction: 1 | -1 | 0) => {
    if (!user) {
      alert('Please log in to vote.');
      return;
    }
    if (busy) return;
    // Toggle: clicking the active direction clears it.
    const newDir = likes === direction ? 0 : direction;
    setBusy(true);
    const prevLikes = likes;
    const prevScore = currentScore;
    // Optimistic update
    setLikes(newDir);
    setCurrentScore((s) => s + (newDir - (prevLikes ?? 0)));
    try {
      const res = await api.vote(targetId, newDir as 1 | 0 | -1);
      setLikes(res.likes);
      setCurrentScore(res.score);
      onVoted?.(res.score, res.likes);
    } catch (e) {
      // Roll back
      setLikes(prevLikes);
      setCurrentScore(prevScore);
      alert((e as Error).message);
    } finally {
      setBusy(false);
    }
  };

  const isUp = likes === 1;
  const isDown = likes === -1;

  if (variant === 'comment') {
    return (
      <div className="flex items-center gap-0.5">
        <button
          onClick={() => handleVote(1)}
          disabled={busy}
          className={cn(
            'p-1 rounded hover:bg-accent transition-colors',
            isUp ? 'text-orange-500' : 'text-muted-foreground hover:text-foreground'
          )}
          aria-label="upvote"
        >
          <ChevronUp className="w-4 h-4" />
        </button>
        <span
          className={cn(
            'text-xs font-semibold tabular-nums min-w-[1.5em] text-center',
            isUp ? 'text-orange-500' : isDown ? 'text-blue-500' : 'text-muted-foreground'
          )}
        >
          {currentScore}
        </span>
        <button
          onClick={() => handleVote(-1)}
          disabled={busy}
          className={cn(
            'p-1 rounded hover:bg-accent transition-colors',
            isDown ? 'text-blue-500' : 'text-muted-foreground hover:text-foreground'
          )}
          aria-label="downvote"
        >
          <ChevronDown className="w-4 h-4" />
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center bg-accent/30 rounded-lg py-1 gap-0.5 min-w-[2.5rem]">
      <button
        onClick={() => handleVote(1)}
        disabled={busy}
        className={cn(
          'p-1.5 rounded-full hover:bg-orange-500/20 transition-colors',
          isUp ? 'bg-orange-500/20 text-orange-500' : 'text-muted-foreground hover:text-foreground'
        )}
        aria-label="upvote"
      >
        <ChevronUp className="w-5 h-5 fill-current" />
      </button>
      <span
        className={cn(
          'text-xs font-bold tabular-nums',
          isUp ? 'text-orange-500' : isDown ? 'text-blue-500' : 'text-foreground'
        )}
      >
        {currentScore}
      </span>
      <button
        onClick={() => handleVote(-1)}
        disabled={busy}
        className={cn(
          'p-1.5 rounded-full hover:bg-blue-500/20 transition-colors',
          isDown ? 'bg-blue-500/20 text-blue-500' : 'text-muted-foreground hover:text-foreground'
        )}
        aria-label="downvote"
      >
        <ChevronDown className="w-5 h-5 fill-current" />
      </button>
    </div>
  );
}
