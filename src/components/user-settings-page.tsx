'use client';
import { useAuth } from '@/lib/auth-context';

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Settings, Shield, Mail, Lock, Trash2, User, Save, Key, CheckCircle2, XCircle, Loader2, Bell } from 'lucide-react';
import { api } from '@/lib/api';
import { cn } from '@/lib/utils';

interface UserSettingsPageProps {
  onClose: () => void;
}

type Section = 'account' | 'preferences' | 'security' | 'privacy' | 'notifications';

export function UserSettingsPage({ onClose }: UserSettingsPageProps) {
  const { user, logout } = useAuth();
  const qc = useQueryClient();
  const [section, setSection] = useState<Section>('account');
  const [msg, setMsg] = useState<{ type: 'success' | 'error'; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  if (!user) return null;

  const sections: { id: Section; label: string; icon: React.ReactNode }[] = [
    { id: 'account', label: 'Account', icon: <User className="w-4 h-4" /> },
    { id: 'preferences', label: 'Preferences', icon: <Settings className="w-4 h-4" /> },
    { id: 'security', label: 'Security', icon: <Shield className="w-4 h-4" /> },
    { id: 'privacy', label: 'Privacy', icon: <Lock className="w-4 h-4" /> },
    { id: 'notifications', label: 'Notifications', icon: <Bell className="w-4 h-4" /> },
  ];

  return (
    <div className="fixed inset-0 z-[100] bg-black/60 flex items-center justify-center p-4" onClick={onClose}>
      <div
        className="bg-card border border-border rounded-xl w-full max-w-4xl p-0 relative max-h-[90vh] overflow-hidden flex"
        onClick={(e) => e.stopPropagation()}
        style={{ minHeight: '600px' }}
      >
        {/* Sidebar */}
        <aside className="w-56 shrink-0 bg-accent/30 border-r border-border p-3">
          <h2 className="text-lg font-bold mb-3">Settings</h2>
          <nav className="space-y-1">
            {sections.map((s) => (
              <button
                key={s.id}
                onClick={() => { setSection(s.id); setMsg(null); }}
                className={cn(
                  'w-full flex items-center gap-2 px-3 py-2 rounded-md text-sm font-semibold transition-colors text-left',
                  section === s.id ? 'bg-orange-500/15 text-orange-600 dark:text-orange-400' : 'hover:bg-accent'
                )}
              >
                {s.icon}
                {s.label}
              </button>
            ))}
          </nav>
          <button
            onClick={onClose}
            className="mt-6 w-full text-xs text-muted-foreground hover:text-foreground text-center"
          >
            Close settings
          </button>
        </aside>

        {/* Main panel */}
        <div className="flex-1 overflow-y-auto p-6">
          {msg && (
            <div className={cn(
              'mb-4 p-3 rounded-lg text-sm flex items-center gap-2',
              msg.type === 'success' ? 'bg-green-500/10 text-green-600 dark:text-green-400 border border-green-500/30' : 'bg-red-500/10 text-red-600 dark:text-red-400 border border-red-500/30'
            )}>
              {msg.type === 'success' ? <CheckCircle2 className="w-4 h-4" /> : <XCircle className="w-4 h-4" />}
              {msg.text}
            </div>
          )}

          {section === 'account' && <AccountSection user={user} setMsg={setMsg} logout={logout} qc={qc} busy={busy} setBusy={setBusy} />}
          {section === 'preferences' && <PreferencesSection setMsg={setMsg} qc={qc} busy={busy} setBusy={setBusy} />}
          {section === 'security' && <SecuritySection user={user} setMsg={setMsg} busy={busy} setBusy={setBusy} />}
          {section === 'privacy' && <PrivacySection setMsg={setMsg} qc={qc} busy={busy} setBusy={setBusy} />}
          {section === 'notifications' && <NotificationsSection setMsg={setMsg} qc={qc} busy={busy} setBusy={setBusy} />}
        </div>
      </div>
    </div>
  );
}

interface SectionProps {
  user: { id: string; username: string; is_admin: boolean; post_karma: number; comment_karma: number };
  setMsg: (msg: { type: 'success' | 'error'; text: string } | null) => void;
  logout: () => void;
  qc: ReturnType<typeof useQueryClient>;
  busy: boolean;
  setBusy: (b: boolean) => void;
}

function AccountSection({ user, setMsg, logout, qc, busy, setBusy }: SectionProps) {
  const [avatar, setAvatar] = useState('');
  const [bannerColor, setBannerColor] = useState('#1a1a1b');
  const [bio, setBio] = useState('');
  const [email, setEmail] = useState('');
  const [deletePassword, setDeletePassword] = useState('');

  // Load existing profile
  useQuery({
    queryKey: ['user', user.username],
    queryFn: async () => {
      const r = await api.getUser(user.username);
      const d = r.data;
      setAvatar(d.avatar || '');
      setBannerColor(d.banner_color || '#1a1a1b');
      setBio(d.bio || '');
      setEmail(d.email || '');
      return r;
    },
  });

  const save = async () => {
    setBusy(true); setMsg(null);
    try {
      await api.updateProfile({ avatar: avatar || undefined, banner_color: bannerColor, bio, email: email || undefined });
      qc.invalidateQueries({ queryKey: ['user', user.username] });
      setMsg({ type: 'success', text: 'Profile updated.' });
    } catch (e) { setMsg({ type: 'error', text: (e as Error).message }); }
    finally { setBusy(false); }
  };

  const del = async () => {
    if (!confirm('Delete your account? This is IRREVERSIBLE. Your posts/comments will show as [deleted].')) return;
    setBusy(true); setMsg(null);
    try {
      await api.deleteAccount(deletePassword);
      setMsg({ type: 'success', text: 'Account deleted.' });
      setTimeout(() => logout(), 1500);
    } catch (e) { setMsg({ type: 'error', text: (e as Error).message }); }
    finally { setBusy(false); }
  };

  return (
    <div className="space-y-5">
      <div>
        <h3 className="text-lg font-bold mb-1">Account Settings</h3>
        <p className="text-xs text-muted-foreground">u/{user.username} · {user.post_karma + user.comment_karma} karma</p>
      </div>

      <div>
        <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Avatar URL</label>
        <input type="url" value={avatar} onChange={(e) => setAvatar(e.target.value)} placeholder="https://…" className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500" />
      </div>

      <div>
        <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Banner color</label>
        <div className="flex gap-2 items-center mt-1">
          <input type="color" value={bannerColor} onChange={(e) => setBannerColor(e.target.value)} className="w-12 h-9 rounded" />
          <input type="text" value={bannerColor} onChange={(e) => setBannerColor(e.target.value)} className="bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500 flex-1" />
        </div>
      </div>

      <div>
        <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Bio (max 200 chars)</label>
        <textarea value={bio} onChange={(e) => setBio(e.target.value)} maxLength={200} rows={3} className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500 resize-y" />
        <p className="text-xs text-muted-foreground mt-1 text-right">{bio.length}/200</p>
      </div>

      <div>
        <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Email</label>
        <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} placeholder="you@example.com" className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500" />
        <p className="text-xs text-muted-foreground mt-1">Changing email will require re-verification.</p>
      </div>

      <div className="flex justify-end">
        <button
          onClick={save}
          disabled={busy}
          className="px-4 py-2 rounded-full text-sm font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50 flex items-center gap-2"
        >
          <Save className="w-4 h-4" /> {busy ? 'Saving…' : 'Save changes'}
        </button>
      </div>

      <div className="border-t border-border pt-5">
        <h4 className="text-base font-semibold text-red-500 mb-2 flex items-center gap-2">
          <Trash2 className="w-4 h-4" /> Delete account
        </h4>
        <p className="text-xs text-muted-foreground mb-2">Permanently delete your account. Posts/comments remain as [deleted].</p>
        <div className="flex gap-2">
          <input type="password" value={deletePassword} onChange={(e) => setDeletePassword(e.target.value)} placeholder="Confirm with password" className="flex-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-red-500" />
          <button onClick={del} disabled={busy || !deletePassword} className="px-4 py-2 rounded-full text-sm font-semibold bg-red-500 hover:bg-red-600 text-white disabled:opacity-50">
            Delete
          </button>
        </div>
      </div>
    </div>
  );
}

function PreferencesSection({ setMsg, qc, busy, setBusy }: Omit<SectionProps, 'user' | 'logout'>) {
  const { data: settings } = useQuery({ queryKey: ['user-settings'], queryFn: api.getSettings });
  const [theme, setTheme] = useState('system');
  const [contentVis, setContentVis] = useState('safe');
  const [defaultSort, setDefaultSort] = useState('hot');
  const [defaultCommentSort, setDefaultCommentSort] = useState('best');

  // Sync from query
  useState(() => {
    if (settings) {
      setTheme(settings.theme);
      setContentVis(settings.content_visibility);
      setDefaultSort(settings.default_sort);
      setDefaultCommentSort(settings.default_comment_sort);
    }
  });

  const save = async () => {
    setBusy(true); setMsg(null);
    try {
      await api.updateSettings({ theme, content_visibility: contentVis, default_sort: defaultSort, default_comment_sort: defaultCommentSort });
      qc.invalidateQueries({ queryKey: ['user-settings'] });
      setMsg({ type: 'success', text: 'Preferences saved.' });
    } catch (e) { setMsg({ type: 'error', text: (e as Error).message }); }
    finally { setBusy(false); }
  };

  return (
    <div className="space-y-5">
      <div>
        <h3 className="text-lg font-bold mb-1">Preferences</h3>
        <p className="text-xs text-muted-foreground">Default display options</p>
      </div>

      <Field label="Theme">
        <select value={theme} onChange={(e) => setTheme(e.target.value)} className="w-full bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm">
          <option value="system">System</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </Field>

      <Field label="Content visibility">
        <select value={contentVis} onChange={(e) => setContentVis(e.target.value)} className="w-full bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm">
          <option value="safe">Safe (hide NSFW)</option>
          <option value="nsfw">Show NSFW</option>
        </select>
      </Field>

      <Field label="Default post sort">
        <select value={defaultSort} onChange={(e) => setDefaultSort(e.target.value)} className="w-full bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm">
          {['hot', 'best', 'new', 'top', 'rising', 'controversial'].map(s => <option key={s} value={s}>{s}</option>)}
        </select>
      </Field>

      <Field label="Default comment sort">
        <select value={defaultCommentSort} onChange={(e) => setDefaultCommentSort(e.target.value)} className="w-full bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm">
          {['best', 'top', 'new', 'controversial', 'old', 'qa'].map(s => <option key={s} value={s}>{s}</option>)}
        </select>
      </Field>

      <div className="flex justify-end">
        <button onClick={save} disabled={busy} className="px-4 py-2 rounded-full text-sm font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50 flex items-center gap-2">
          <Save className="w-4 h-4" /> {busy ? 'Saving…' : 'Save preferences'}
        </button>
      </div>
    </div>
  );
}

function SecuritySection({ user, setMsg, busy, setBusy }: SectionProps) {
  const { data: status } = useQuery({ queryKey: ['2fa-status'], queryFn: api.get2FAStatus });
  const { data: settings } = useQuery({ queryKey: ['user-settings'], queryFn: api.getSettings });
  const [secret, setSecret] = useState<string | null>(null);
  const [qrUrl, setQrUrl] = useState<string | null>(null);
  const [enableCode, setEnableCode] = useState('');
  const [backupCodes, setBackupCodes] = useState<string[] | null>(null);
  const [disableCode, setDisableCode] = useState('');
  const [emailToSend, setEmailToSend] = useState('');

  const startEnable = async () => {
    setBusy(true); setMsg(null);
    try {
      const r = await api.enable2FA();
      setSecret(r.secret || null);
      setQrUrl(r.qr_url || null);
    } catch (e) { setMsg({ type: 'error', text: (e as Error).message }); }
    finally { setBusy(false); }
  };

  const confirmEnable = async () => {
    setBusy(true); setMsg(null);
    try {
      const r = await api.enable2FA(enableCode);
      if (r.backup_codes) setBackupCodes(r.backup_codes);
      setSecret(null); setQrUrl(null); setEnableCode('');
      setMsg({ type: 'success', text: '2FA enabled. Save your backup codes!' });
    } catch (e) { setMsg({ type: 'error', text: (e as Error).message }); }
    finally { setBusy(false); }
  };

  const doDisable = async () => {
    setBusy(true); setMsg(null);
    try {
      await api.disable2FA(disableCode);
      setDisableCode('');
      setMsg({ type: 'success', text: '2FA disabled.' });
    } catch (e) { setMsg({ type: 'error', text: (e as Error).message }); }
    finally { setBusy(false); }
  };

  const sendVerification = async () => {
    setBusy(true); setMsg(null);
    try {
      const r = await api.sendEmailVerification(emailToSend || undefined);
      setMsg({ type: 'success', text: `Verification email sent to ${r.email}. Check /home/z/my-project/emails/ in dev.` });
    } catch (e) { setMsg({ type: 'error', text: (e as Error).message }); }
    finally { setBusy(false); }
  };

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-bold mb-1">Security</h3>
        <p className="text-xs text-muted-foreground">2FA, email verification, password reset</p>
      </div>

      {/* Email verification */}
      <div className="border border-border rounded-lg p-4">
        <h4 className="font-semibold mb-1 flex items-center gap-2">
          <Mail className="w-4 h-4" /> Email Verification
          {settings?.email_verified && <span className="text-xs text-green-600 dark:text-green-400 bg-green-500/15 px-2 py-0.5 rounded">Verified</span>}
          {!settings?.email_verified && <span className="text-xs text-yellow-700 bg-yellow-500/15 px-2 py-0.5 rounded">Unverified</span>}
        </h4>
        <p className="text-xs text-muted-foreground mb-2">Verify your email to enable password recovery and email notifications.</p>
        <div className="flex gap-2">
          <input type="email" value={emailToSend} onChange={(e) => setEmailToSend(e.target.value)} placeholder="you@example.com (optional)" className="flex-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500" />
          <button onClick={sendVerification} disabled={busy} className="px-4 py-2 rounded-full text-sm font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50">
            Send verification
          </button>
        </div>
      </div>

      {/* 2FA */}
      <div className="border border-border rounded-lg p-4">
        <h4 className="font-semibold mb-1 flex items-center gap-2">
          <Shield className="w-4 h-4" /> Two-Factor Authentication (TOTP)
          {status?.two_factor_enabled && <span className="text-xs text-green-600 dark:text-green-400 bg-green-500/15 px-2 py-0.5 rounded">Enabled</span>}
          {!status?.two_factor_enabled && <span className="text-xs text-muted-foreground bg-accent px-2 py-0.5 rounded">Disabled</span>}
        </h4>

        {backupCodes && (
          <div className="bg-yellow-500/10 border border-yellow-500/30 rounded-lg p-3 my-3">
            <p className="text-xs font-semibold text-yellow-700 mb-2">Backup codes (save these — shown only once):</p>
            <div className="grid grid-cols-2 gap-2 text-xs font-mono">
              {backupCodes.map((c, i) => <div key={i} className="bg-card px-2 py-1 rounded">{c}</div>)}
            </div>
          </div>
        )}

        {!status?.two_factor_enabled && !secret && (
          <button onClick={startEnable} disabled={busy} className="px-4 py-2 rounded-full text-sm font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50">
            Enable 2FA
          </button>
        )}

        {secret && qrUrl && (
          <div className="space-y-3">
            <p className="text-xs text-muted-foreground">Scan this QR URL in your authenticator app (Authy, Google Authenticator, 1Password, etc.):</p>
            <div className="bg-accent/30 border border-border rounded-lg p-3 font-mono text-xs break-all">{qrUrl}</div>
            <p className="text-xs text-muted-foreground">Or enter manually: <code className="bg-accent/30 px-1 py-0.5 rounded">{secret}</code></p>
            <div className="flex gap-2">
              <input value={enableCode} onChange={(e) => setEnableCode(e.target.value)} placeholder="6-digit code from app" maxLength={6} className="flex-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm font-mono outline-none focus:border-orange-500" />
              <button onClick={confirmEnable} disabled={busy || enableCode.length !== 6} className="px-4 py-2 rounded-full text-sm font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50">
                Verify & enable
              </button>
            </div>
          </div>
        )}

        {status?.two_factor_enabled && (
          <div className="flex gap-2 mt-2">
            <input value={disableCode} onChange={(e) => setDisableCode(e.target.value)} placeholder="6-digit code or backup code" className="flex-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm font-mono outline-none focus:border-orange-500" />
            <button onClick={doDisable} disabled={busy || !disableCode} className="px-4 py-2 rounded-full text-sm font-semibold bg-red-500 hover:bg-red-600 text-white disabled:opacity-50">
              Disable 2FA
            </button>
          </div>
        )}
      </div>

      {/* Password reset */}
      <div className="border border-border rounded-lg p-4">
        <h4 className="font-semibold mb-1 flex items-center gap-2">
          <Key className="w-4 h-4" /> Reset Password
        </h4>
        <p className="text-xs text-muted-foreground mb-2">Trigger a password reset. In dev, the reset link is written to /home/z/my-project/emails/.</p>
        <button onClick={async () => {
          setBusy(true); setMsg(null);
          try {
            const r = await api.requestPasswordReset(user.username);
            setMsg({ type: 'success', text: r.token ? `Reset link sent. Token: ${r.token}` : 'Reset link sent if account exists.' });
          } catch (e) { setMsg({ type: 'error', text: (e as Error).message }); }
          finally { setBusy(false); }
        }} disabled={busy} className="px-4 py-2 rounded-full text-sm font-semibold border border-border hover:bg-accent">
          Send reset link
        </button>
      </div>
    </div>
  );
}

function PrivacySection({ setMsg, qc, busy, setBusy }: Omit<SectionProps, 'user' | 'logout'>) {
  const { data: settings } = useQuery({ queryKey: ['user-settings'], queryFn: api.getSettings });
  const [showOnline, setShowOnline] = useState(true);
  const [showPosts, setShowPosts] = useState(true);
  const [showComments, setShowComments] = useState(true);

  useState(() => {
    if (settings) {
      setShowOnline(settings.show_online_status);
      setShowPosts(settings.show_post_history);
      setShowComments(settings.show_comment_history);
    }
  });

  const save = async () => {
    setBusy(true); setMsg(null);
    try {
      await api.updateSettings({
        show_online_status: showOnline,
        show_post_history: showPosts,
        show_comment_history: showComments,
      });
      qc.invalidateQueries({ queryKey: ['user-settings'] });
      setMsg({ type: 'success', text: 'Privacy settings saved.' });
    } catch (e) { setMsg({ type: 'error', text: (e as Error).message }); }
    finally { setBusy(false); }
  };

  return (
    <div className="space-y-5">
      <div>
        <h3 className="text-lg font-bold mb-1">Privacy</h3>
        <p className="text-xs text-muted-foreground">Control who sees your activity</p>
      </div>

      <Toggle label="Show online status" desc="Display when you're active on freeddit" checked={showOnline} onChange={setShowOnline} />
      <Toggle label="Show post history" desc="Allow others to see posts you've made" checked={showPosts} onChange={setShowPosts} />
      <Toggle label="Show comment history" desc="Allow others to see comments you've made" checked={showComments} onChange={setShowComments} />

      <div className="flex justify-end">
        <button onClick={save} disabled={busy} className="px-4 py-2 rounded-full text-sm font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50 flex items-center gap-2">
          <Save className="w-4 h-4" /> {busy ? 'Saving…' : 'Save privacy settings'}
        </button>
      </div>
    </div>
  );
}

function NotificationsSection({ setMsg, qc, busy, setBusy }: Omit<SectionProps, 'user' | 'logout'>) {
  const { data: settings } = useQuery({ queryKey: ['user-settings'], queryFn: api.getSettings });
  const [onPm, setOnPm] = useState(true);
  const [onMention, setOnMention] = useState(false);
  const [onReply, setOnReply] = useState(true);

  useState(() => {
    if (settings) {
      setOnPm(settings.email_on_pm);
      setOnMention(settings.email_on_mention);
      setOnReply(settings.email_on_reply);
    }
  });

  const save = async () => {
    setBusy(true); setMsg(null);
    try {
      await api.updateSettings({
        email_on_pm: onPm, email_on_mention: onMention, email_on_reply: onReply,
      });
      qc.invalidateQueries({ queryKey: ['user-settings'] });
      setMsg({ type: 'success', text: 'Notification settings saved.' });
    } catch (e) { setMsg({ type: 'error', text: (e as Error).message }); }
    finally { setBusy(false); }
  };

  return (
    <div className="space-y-5">
      <div>
        <h3 className="text-lg font-bold mb-1">Notifications</h3>
        <p className="text-xs text-muted-foreground">Email notifications (requires verified email)</p>
      </div>
      <Toggle label="Private messages" desc="Email me when I receive a private message" checked={onPm} onChange={setOnPm} />
      <Toggle label="Mentions" desc="Email me when someone mentions u/me" checked={onMention} onChange={setOnMention} />
      <Toggle label="Comment replies" desc="Email me when someone replies to my comment" checked={onReply} onChange={setOnReply} />
      <div className="flex justify-end">
        <button onClick={save} disabled={busy} className="px-4 py-2 rounded-full text-sm font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50 flex items-center gap-2">
          <Save className="w-4 h-4" /> {busy ? 'Saving…' : 'Save'}
        </button>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">{label}</label>
      <div className="mt-1">{children}</div>
    </div>
  );
}

function Toggle({ label, desc, checked, onChange }: { label: string; desc: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <div className="flex items-start justify-between p-3 bg-accent/20 rounded-lg">
      <div className="flex-1">
        <p className="font-semibold text-sm">{label}</p>
        <p className="text-xs text-muted-foreground">{desc}</p>
      </div>
      <button
        onClick={() => onChange(!checked)}
        className={cn(
          'w-12 h-6 rounded-full transition-colors relative shrink-0 ml-3',
          checked ? 'bg-orange-500' : 'bg-accent'
        )}
      >
        <span className={cn('absolute top-0.5 w-5 h-5 bg-white rounded-full transition-transform', checked ? 'left-6' : 'left-0.5')} />
      </button>
    </div>
  );
}
