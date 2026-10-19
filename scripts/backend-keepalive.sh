#!/bin/bash
cd /home/z/my-project
while true; do
  echo "[$(date)] Starting backend..."
  DATABASE_URL="sqlite:///home/z/my-project/freeddit.db" PORT=3001 \
    /home/z/my-project/backend-rust/target/release/freeddit-backend > /home/z/my-project/backend.log 2>&1
  echo "[$(date)] Backend exited with code $?, restarting in 2s..."
  sleep 2
done
