'use client';

import { cn } from '@/lib/utils';
import { colorFromString, initials } from '@/lib/format';

interface SubredditAvatarProps {
  name: string;
  size?: number;
  className?: string;
}

export function SubredditAvatar({ name, size = 32, className }: SubredditAvatarProps) {
  const bg = colorFromString(name);
  return (
    <div
      className={cn(
        'rounded-full flex items-center justify-center font-bold text-white shrink-0',
        className
      )}
      style={{
        width: size,
        height: size,
        backgroundColor: bg,
        fontSize: size * 0.4,
      }}
      aria-label={`r/${name}`}
    >
      {initials(name)}
    </div>
  );
}

interface UserAvatarProps {
  name: string;
  size?: number;
  className?: string;
}

export function UserAvatar({ name, size = 32, className }: UserAvatarProps) {
  const bg = colorFromString(name);
  return (
    <div
      className={cn(
        'rounded-full flex items-center justify-center font-bold text-white shrink-0',
        className
      )}
      style={{
        width: size,
        height: size,
        backgroundColor: bg,
        fontSize: size * 0.4,
      }}
      aria-label={`u/${name}`}
    >
      {initials(name)}
    </div>
  );
}
