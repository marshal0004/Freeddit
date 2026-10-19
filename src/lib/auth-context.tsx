'use client';

import { createContext, useContext, useEffect, useState, ReactNode, useCallback } from 'react';
import { api, AuthUser, getToken, setToken, getStoredUser } from './api';

interface AuthContextValue {
  user: AuthUser | null;
  loading: boolean;
  login: (username: string, password: string) => Promise<void>;
  register: (username: string, password: string, email?: string) => Promise<void>;
  logout: () => void;
  refreshUser: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | undefined>(undefined);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Hydrate from localStorage on mount
    const stored = getStoredUser();
    if (stored) setUser(stored);
    setLoading(false);
  }, []);

  const login = useCallback(async (username: string, password: string) => {
    const res = await api.login(username, password);
    setToken(res.access_token, res.user);
    setUser(res.user);
  }, []);

  const register = useCallback(async (username: string, password: string, email?: string) => {
    const res = await api.register(username, password, email);
    setToken(res.access_token, res.user);
    setUser(res.user);
  }, []);

  const logout = useCallback(() => {
    setToken(null, null);
    setUser(null);
  }, []);

  const refreshUser = useCallback(async () => {
    if (!getToken()) return;
    try {
      const me = await api.me();
      setUser((prev) => prev ? { ...prev, ...me } : prev);
    } catch {
      setToken(null, null);
      setUser(null);
    }
  }, []);

  return (
    <AuthContext.Provider value={{ user, loading, login, register, logout, refreshUser }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used inside AuthProvider');
  return ctx;
}
