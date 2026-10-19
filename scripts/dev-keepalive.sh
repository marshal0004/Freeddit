#!/bin/bash
# Keep the Next.js dev server alive forever
cd /home/z/my-project
while true; do
  echo "[$(date)] Starting dev server..."
  bun run dev > /home/z/my-project/dev.log 2>&1
  echo "[$(date)] Dev server exited with code $?, restarting in 3s..."
  sleep 3
done
