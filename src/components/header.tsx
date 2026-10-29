'use client';

import { useState, useRef, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Search, Bell, Plus, MessageSquare, LogOut, User as UserIcon, Sun, Moon, Menu, Settings as SettingsIcon } from 'lucide-react';
import { useAuth } from '@/lib/auth-context';
import { api } from '@/lib/api';
import { UserAvatar } from './subreddit-avatar';
import { cn } from '@/lib/utils';

interface HeaderProps {
  onSearch: (q: string) => void;
  onCreatePost: () => void;
  onOpenUser: (name: string) => void;
  onToggleSidebar: () => void;
  onOpenLogin: () => void;
  onOpenRegister: () => void;
  onOpenInbox: () => void;
  onOpenSettings: () => void;
  theme: 'light' | 'dark';
  onToggleTheme: () => void;
}

export function Header({
  onSearch, onCreatePost, onOpenUser, onToggleSidebar, onOpenLogin, onOpenRegister,
  onOpenInbox, onOpenSettings,
  theme, onToggleTheme,
}: HeaderProps) {
  const { user, logout } = useAuth();
  const [q, setQ] = useState('');
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  // Poll for unread notification count
  const { data: notifData } = useQuery({
    queryKey: ['notifications', 'unread'],
    queryFn: () => api.listNotifications(true),
    enabled: !!user,
    refetchInterval: 30_000,
  });
  const unreadCount = notifData?.unread_count;

  useEffect(() => {
    function onClick(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setMenuOpen(false);
    }
    document.addEventListener('mousedown', onClick);
    return () => document.removeEventListener('mousedown', onClick);
  }, []);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    if (q.trim()) onSearch(q.trim());
  };

  return (
    <header className="sticky top-0 z-50 bg-background/95 backdrop-blur border-b border-border h-12 flex items-center gap-2 px-3">
      <button
        onClick={onToggleSidebar}
        className="p-2 rounded hover:bg-accent lg:hidden"
        aria-label="Toggle sidebar"
      >
        <Menu className="w-5 h-5" />
      </button>

      {/* Logo */}
      <a
        href="/"
        onClick={(e) => { e.preventDefault(); window.location.hash = ''; window.scrollTo(0, 0); }}
        className="flex items-center gap-2 font-bold text-lg shrink-0"
      >
        <img src="/freeddit-logo.svg" alt="freeddit" className="w-8 h-8 rounded-lg" />
        <span className="hidden sm:inline">freeddit</span>
      </a>

      {/* Search */}
      <form onSubmit={submit} className="flex-1 max-w-2xl mx-2">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <input
            type="search"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Search Reddit"
            className="w-full bg-accent/40 border border-transparent focus:border-orange-500 focus:bg-background rounded-full py-1.5 pl-10 pr-4 text-sm outline-none transition-colors"
          />
        </div>
      </form>

      {/* Right side actions */}
      <div className="flex items-center gap-1 ml-auto">
        <button
          onClick={onToggleTheme}
          className="p-2 rounded-full hover:bg-accent"
          aria-label="Toggle theme"
        >
          {theme === 'dark' ? <Sun className="w-5 h-5" /> : <Moon className="w-5 h-5" />}
        </button>

        {user ? (
          <>
            <button
              onClick={onCreatePost}
              className="hidden sm:flex items-center gap-1 px-3 py-1.5 rounded-full bg-orange-500 hover:bg-orange-600 text-white text-sm font-semibold transition-colors"
            >
              <Plus className="w-4 h-4" />
              Create
            </button>
            <button
              onClick={onOpenInbox}
              className="p-2 rounded-full hover:bg-accent relative" aria-label="Notifications"
            >
              <Bell className="w-5 h-5" />
              {(unreadCount ?? 0) > 0 && (
                <span className="absolute top-0 right-0 bg-orange-500 text-white text-[10px] font-bold rounded-full px-1.5 py-0.5 leading-none min-w-[16px] text-center">
                  {unreadCount! > 99 ? '99+' : unreadCount}
                </span>
              )}
            </button>
            <div className="relative" ref={menuRef}>
              <button
                onClick={() => setMenuOpen((v) => !v)}
                className="flex items-center gap-1 p-1 rounded-full hover:bg-accent"
              >
                <UserAvatar name={user.username} size={28} />
              </button>
              {menuOpen && (
                <div className="absolute right-0 top-full mt-1 w-56 bg-card border border-border rounded-lg shadow-lg overflow-hidden">
                  <div className="px-3 py-2 border-b border-border">
                    <div className="font-semibold text-sm">u/{user.username}</div>
                    <div className="text-xs text-muted-foreground">
                      {user.post_karma + user.comment_karma} karma
                    </div>
                  </div>
                  <button
                    onClick={() => {
                      setMenuOpen(false);
                      onOpenUser(user.username);
                    }}
                    className="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-accent text-left"
                  >
                    <UserIcon className="w-4 h-4" /> Profile
                  </button>
                  <button
                    onClick={() => {
                      setMenuOpen(false);
                      onOpenSettings();
                    }}
                    className="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-accent text-left"
                  >
                    <SettingsIcon className="w-4 h-4" /> User Settings
                  </button>
                  <button
                    onClick={() => {
                      setMenuOpen(false);
                      logout();
                    }}
                    className="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-accent text-left text-red-500"
                  >
                    <LogOut className="w-4 h-4" /> Log Out
                  </button>
                </div>
              )}
            </div>
          </>
        ) : (
          <>
            <button
              onClick={onOpenLogin}
              className="px-3 py-1.5 rounded-full bg-orange-500 hover:bg-orange-600 text-white text-sm font-semibold transition-colors"
            >
              Log In
            </button>
            <button
              onClick={onOpenRegister}
              className="hidden sm:inline px-3 py-1.5 rounded-full hover:bg-accent text-foreground text-sm font-semibold transition-colors"
            >
              Sign Up
            </button>
          </>
        )}
      </div>
    </header>
  );
}
