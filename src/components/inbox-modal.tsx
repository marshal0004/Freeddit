'use client';
import { useAuth } from '@/lib/auth-context';

import { useState, useRef, useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Bell, MessageSquare, CheckCheck, X, Send, Inbox as InboxIcon, Trash2 } from 'lucide-react';
import { api, Notification } from '@/lib/api';
import { timeAgo } from '@/lib/format';
import { UserAvatar } from './subreddit-avatar';
import { Markdown } from './markdown';
import { cn } from '@/lib/utils';

interface InboxModalProps {
  onClose: () => void;
  onOpenPost: (postId: string) => void;
  onOpenUser: (name: string) => void;
}

type Tab = 'notifications' | 'messages';

export function InboxModal({ onClose, onOpenPost, onOpenUser }: InboxModalProps) {
  const { user } = useAuth();
  const qc = useQueryClient();
  const [tab, setTab] = useState<Tab>('notifications');
  const [openConv, setOpenConv] = useState<string | null>(null);

  if (!user) return null;

  return (
    <div className="fixed inset-0 z-[100] bg-black/60 flex items-center justify-center p-4" onClick={onClose}>
      <div
        className="bg-card border border-border rounded-xl w-full max-w-3xl p-0 relative max-h-[85vh] overflow-hidden flex flex-col"
        onClick={(e) => e.stopPropagation()}
        style={{ minHeight: '500px' }}
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border p-4">
          <div className="flex gap-2">
            <button
              onClick={() => setTab('notifications')}
              className={cn('flex items-center gap-2 px-4 py-2 rounded-full text-sm font-semibold',
                tab === 'notifications' ? 'bg-orange-500/15 text-orange-600 dark:text-orange-400' : 'hover:bg-accent text-muted-foreground')}
            >
              <Bell className="w-4 h-4" /> Notifications
            </button>
            <button
              onClick={() => setTab('messages')}
              className={cn('flex items-center gap-2 px-4 py-2 rounded-full text-sm font-semibold',
                tab === 'messages' ? 'bg-orange-500/15 text-orange-600 dark:text-orange-400' : 'hover:bg-accent text-muted-foreground')}
            >
              <MessageSquare className="w-4 h-4" /> Messages
            </button>
          </div>
          <button onClick={onClose} className="p-1.5 rounded-full hover:bg-accent" aria-label="Close"><X className="w-4 h-4" /></button>
        </div>

        <div className="flex-1 overflow-y-auto">
          {tab === 'notifications' && (
            <NotificationsTab onOpenPost={onOpenPost} onOpenUser={onOpenUser} qc={qc} />
          )}
          {tab === 'messages' && (
            <MessagesTab user={user} openConv={openConv} setOpenConv={setOpenConv} onOpenUser={onOpenUser} qc={qc} />
          )}
        </div>
      </div>
    </div>
  );
}

function NotificationsTab({ onOpenPost, onOpenUser, qc }: { onOpenPost: (id: string) => void; onOpenUser: (n: string) => void; qc: ReturnType<typeof useQueryClient> }) {
  const { data, isLoading } = useQuery({ queryKey: ['notifications'], queryFn: () => api.listNotifications() });

  const markAll = useMutation({
    mutationFn: api.markAllNotificationsRead,
    onSuccess: () => qc.invalidateQueries({ queryKey: ['notifications'] }),
  });

  const markOne = useMutation({
    mutationFn: (id: string) => api.markNotificationRead(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['notifications'] }),
  });

  if (isLoading) return <div className="p-8 text-center text-muted-foreground">Loading…</div>;

  const notifications = data?.notifications || [];
  if (notifications.length === 0) {
    return (
      <div className="p-8 text-center text-muted-foreground">
        <InboxIcon className="w-12 h-12 mx-auto mb-3 opacity-40" />
        <p>No notifications yet.</p>
        <p className="text-xs mt-1">You'll see replies, mentions, and awards here.</p>
      </div>
    );
  }

  return (
    <div>
      <div className="flex justify-end p-2 border-b border-border">
        <button
          onClick={() => markAll.mutate()}
          className="text-xs font-semibold text-orange-600 dark:text-orange-400 hover:underline flex items-center gap-1"
        >
          <CheckCheck className="w-3 h-3" /> Mark all as read
        </button>
      </div>
      {notifications.map((n: Notification) => (
        <div
          key={n.id}
          className={cn(
            'p-3 border-b border-border flex gap-3 cursor-pointer hover:bg-accent/30 transition-colors',
            !n.is_read && 'bg-orange-500/5'
          )}
          onClick={() => {
            markOne.mutate(n.id);
            if (n.target_id && n.target_type === 'post') onOpenPost(n.target_id);
            else if (n.target_id && n.target_type === 'comment') onOpenPost(n.target_id); // For comments, opening the post would require a lookup; we treat the target_id as the comment's post
            if (n.actor_username) onOpenUser(n.actor_username);
          }}
        >
          <UserAvatar name={n.actor_username || '?'} size={32} />
          <div className="flex-1 min-w-0">
            <p className="text-sm">
              <button onClick={(e) => { e.stopPropagation(); if (n.actor_username) onOpenUser(n.actor_username); }} className="font-semibold hover:underline">
                u/{n.actor_username || 'unknown'}
              </button>{' '}
              <span className="text-muted-foreground">
                {n.kind === 'comment_reply' && 'replied to your comment'}
                {n.kind === 'private_message' && 'sent you a private message'}
                {n.kind === 'mention' && 'mentioned you'}
                {n.kind === 'award' && 'gave you an award'}
                {n.kind === 'mod_action' && 'performed a mod action'}
              </span>
            </p>
            {n.body && <p className="text-xs text-muted-foreground mt-1 line-clamp-2">{n.body}</p>}
            <p className="text-xs text-muted-foreground mt-0.5">{timeAgo(n.created_at)}</p>
          </div>
          {!n.is_read && <div className="w-2 h-2 rounded-full bg-orange-500 mt-2 shrink-0" />}
        </div>
      ))}
    </div>
  );
}

function MessagesTab({ user, openConv, setOpenConv, onOpenUser, qc }: {
  user: { id: string; username: string };
  openConv: string | null;
  setOpenConv: (id: string | null) => void;
  onOpenUser: (n: string) => void;
  qc: ReturnType<typeof useQueryClient>;
}) {
  // List conversations
  const { data: convData, isLoading } = useQuery({
    queryKey: ['conversations'],
    queryFn: api.listConversations,
    enabled: !openConv,
  });

  // Get open conversation messages
  const { data: convMessages } = useQuery({
    queryKey: ['conversation', openConv],
    queryFn: () => api.getConversation(openConv!),
    enabled: !!openConv,
  });

  const [newMessage, setNewMessage] = useState('');
  const sendMut = useMutation({
    mutationFn: () => api.sendMessage(openConv!, newMessage),
    onSuccess: () => {
      setNewMessage('');
      qc.invalidateQueries({ queryKey: ['conversation', openConv] });
      qc.invalidateQueries({ queryKey: ['conversations'] });
    },
  });

  const startNewConvMut = useMutation({
    mutationFn: (data: { to: string; body: string }) => api.startConversation({ to: data.to, body: data.body }),
    onSuccess: (r) => {
      qc.invalidateQueries({ queryKey: ['conversations'] });
      setOpenConv(r.conversation_id);
    },
  });

  const [showNewConv, setShowNewConv] = useState(false);
  const [newTo, setNewTo] = useState('');
  const [newBody, setNewBody] = useState('');

  const deleteConvMut = useMutation({
    mutationFn: (id: string) => api.deleteConversation(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['conversations'] });
      setOpenConv(null);
    },
  });

  if (openConv) {
    const messages = convMessages?.messages || [];
    const otherUser = messages.find(m => !m.is_mine)?.sender_name || '';
    return (
      <div className="flex flex-col h-full">
        <div className="flex items-center justify-between border-b border-border p-3">
          <button onClick={() => setOpenConv(null)} className="text-xs text-muted-foreground hover:text-foreground">← Back</button>
          <button onClick={() => onOpenUser(otherUser)} className="font-semibold text-sm hover:underline">u/{otherUser}</button>
          <button onClick={() => deleteConvMut.mutate(openConv)} className="text-xs text-red-500 hover:underline flex items-center gap-1">
            <Trash2 className="w-3 h-3" /> Delete
          </button>
        </div>
        <div className="flex-1 overflow-y-auto p-3 space-y-2">
          {messages.map(m => (
            <div key={m.id} className={cn('flex', m.is_mine ? 'justify-end' : 'justify-start')}>
              <div className={cn('max-w-[70%] rounded-lg p-2', m.is_mine ? 'bg-orange-500 text-white' : 'bg-accent')}>
                <Markdown content={m.body} className="!text-inherit" />
                <p className={cn('text-[10px] mt-1', m.is_mine ? 'text-white/70' : 'text-muted-foreground')}>{timeAgo(m.created_at)}</p>
              </div>
            </div>
          ))}
        </div>
        <div className="border-t border-border p-2 flex gap-2">
          <input
            value={newMessage}
            onChange={(e) => setNewMessage(e.target.value)}
            placeholder="Type a message…"
            className="flex-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500"
            onKeyDown={(e) => { if (e.key === 'Enter' && newMessage.trim()) sendMut.mutate(); }}
          />
          <button onClick={() => sendMut.mutate()} disabled={sendMut.isLoading || !newMessage.trim()} className="px-4 py-2 rounded-full text-sm font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50 flex items-center gap-1">
            <Send className="w-4 h-4" /> Send
          </button>
        </div>
      </div>
    );
  }

  if (showNewConv) {
    return (
      <div className="p-4 space-y-3">
        <button onClick={() => setShowNewConv(false)} className="text-xs text-muted-foreground hover:text-foreground">← Back to conversations</button>
        <h3 className="text-lg font-bold">New message</h3>
        <div>
          <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">To (username)</label>
          <input value={newTo} onChange={(e) => setNewTo(e.target.value)} placeholder="username" className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500" />
        </div>
        <div>
          <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Message</label>
          <textarea value={newBody} onChange={(e) => setNewBody(e.target.value)} rows={4} placeholder="Your message…" className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500 resize-y" />
        </div>
        <button
          onClick={() => { if (newTo.trim() && newBody.trim()) startNewConvMut.mutate({ to: newTo, body: newBody }); }}
          disabled={startNewConvMut.isLoading || !newTo.trim() || !newBody.trim()}
          className="px-4 py-2 rounded-full text-sm font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50 flex items-center gap-2"
        >
          <Send className="w-4 h-4" /> Send
        </button>
      </div>
    );
  }

  if (isLoading) return <div className="p-8 text-center text-muted-foreground">Loading…</div>;

  const convs = convData?.conversations || [];
  if (convs.length === 0) {
    return (
      <div className="p-8 text-center text-muted-foreground">
        <MessageSquare className="w-12 h-12 mx-auto mb-3 opacity-40" />
        <p>No messages yet.</p>
        <button onClick={() => setShowNewConv(true)} className="mt-3 px-4 py-2 rounded-full bg-orange-500 text-white text-sm font-semibold hover:bg-orange-600">
          Send a message
        </button>
      </div>
    );
  }

  return (
    <div>
      <div className="flex justify-end p-2 border-b border-border">
        <button onClick={() => setShowNewConv(true)} className="text-xs font-semibold text-orange-600 dark:text-orange-400 hover:underline">
          + New message
        </button>
      </div>
      {convs.map(c => (
        <button
          key={c.id}
          onClick={() => setOpenConv(c.id)}
          className="w-full p-3 border-b border-border hover:bg-accent/30 transition-colors text-left flex gap-3"
        >
          <UserAvatar name={c.other_participants[0] || '?'} size={36} />
          <div className="flex-1 min-w-0">
            <div className="flex items-center justify-between">
              <p className="font-semibold text-sm">u/{c.other_participants.join(', ')}</p>
              <p className="text-xs text-muted-foreground">{timeAgo(c.last_message_at)}</p>
            </div>
            <p className="text-xs text-muted-foreground truncate mt-0.5">{c.last_message_preview}</p>
          </div>
          {c.unread_count > 0 && (
            <span className="bg-orange-500 text-white text-xs font-bold px-2 py-0.5 rounded-full">{c.unread_count}</span>
          )}
        </button>
      ))}
    </div>
  );
}
