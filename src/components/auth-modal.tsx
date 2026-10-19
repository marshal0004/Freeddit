'use client';

import { useState } from 'react';
import { X } from 'lucide-react';
import { useAuth } from '@/lib/auth-context';
import { cn } from '@/lib/utils';

interface AuthModalProps {
  mode: 'login' | 'register';
  onClose: () => void;
  onSwitchMode: (mode: 'login' | 'register') => void;
}

export function AuthModal({ mode, onClose, onSwitchMode }: AuthModalProps) {
  const { login, register } = useAuth();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [email, setEmail] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      if (mode === 'login') await login(username, password);
      else await register(username, password, email || undefined);
      onClose();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-[100] bg-black/60 flex items-center justify-center p-4"
      onClick={onClose}
    >
      <div
        className="bg-card border border-border rounded-xl w-full max-w-md p-6 relative"
        onClick={(e) => e.stopPropagation()}
      >
        <button
          onClick={onClose}
          className="absolute top-3 right-3 p-1.5 rounded-full hover:bg-accent"
          aria-label="Close"
        >
          <X className="w-4 h-4" />
        </button>

        <div className="flex items-center gap-2 mb-4">
          <div className="w-10 h-10 rounded-full bg-gradient-to-br from-orange-500 to-orange-600 flex items-center justify-center text-white">
            <svg viewBox="0 0 24 24" className="w-6 h-6 fill-current">
              <path d="M12 0a12 12 0 1 0 0 24 12 12 0 0 0 0-24zm5.01 4.74a1.36 1.36 0 1 1 0 2.72 1.36 1.36 0 0 1 0-2.72zm-2.65 1.93a1.36 1.36 0 1 1-2.65.6c-.06-.27-.06-.55 0-.82-.91-.2-1.66.06-2.32.65.55.27.91.83.91 1.5 0 .91-.74 1.65-1.65 1.65-.65 0-1.21-.37-1.48-.91-.46.84-.7 1.85-.7 2.96 0 3.36 2.7 6.07 6.06 6.07s6.07-2.71 6.07-6.07c0-1.66-.66-3.16-1.73-4.27a1.65 1.65 0 0 1-1.48.91c-.91 0-1.65-.74-1.65-1.65 0-.83.62-1.52 1.42-1.63a1.36 1.36 0 0 1 1.27-1.07zm-2.36 4.36c.5 0 .91.41.91.91s-.41.91-.91.91-.91-.41-.91-.91.41-.91.91-.91zm5 1.82a1.36 1.36 0 1 1-1.36 1.36 1.36 1.36 0 0 1 1.36-1.36z"/>
            </svg>
          </div>
          <h2 className="text-xl font-bold">
            {mode === 'login' ? 'Log in' : 'Sign up'}
          </h2>
        </div>

        {error && (
          <div className="bg-red-500/10 border border-red-500/30 text-red-600 dark:text-red-400 text-sm rounded-lg p-3 mb-4">
            {error}
          </div>
        )}

        <form onSubmit={submit} className="space-y-3">
          <div>
            <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Username</label>
            <input
              type="text"
              required
              minLength={3}
              maxLength={20}
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500"
              placeholder="Choose a username"
              autoFocus
            />
          </div>
          {mode === 'register' && (
            <div>
              <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Email (optional)</label>
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500"
                placeholder="you@example.com"
              />
            </div>
          )}
          <div>
            <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Password</label>
            <input
              type="password"
              required
              minLength={6}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500"
              placeholder="At least 6 characters"
            />
          </div>
          <button
            type="submit"
            disabled={busy}
            className="w-full bg-orange-500 hover:bg-orange-600 text-white font-semibold py-2 rounded-lg disabled:opacity-50"
          >
            {busy ? 'Please wait…' : mode === 'login' ? 'Log In' : 'Sign Up'}
          </button>
        </form>

        <p className="text-xs text-center mt-4 text-muted-foreground">
          {mode === 'login' ? "Don't have an account? " : 'Already have an account? '}
          <button
            onClick={() => onSwitchMode(mode === 'login' ? 'register' : 'login')}
            className="text-orange-600 dark:text-orange-400 font-semibold hover:underline"
          >
            {mode === 'login' ? 'Sign up' : 'Log in'}
          </button>
        </p>
      </div>
    </div>
  );
}
