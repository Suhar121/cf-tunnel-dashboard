#!/bin/bash
echo "=========================================================="
echo "   Starting Cloudflare Tunnel Central (Rust + Next.js)     "
echo "=========================================================="

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

echo "[1/2] Starting Rust Backend on http://127.0.0.1:8080..."
(cd "$DIR/backend" && cargo run) &
BACKEND_PID=$!

sleep 2

echo "[2/2] Starting Next.js Frontend on http://localhost:3000..."
(cd "$DIR/frontend" && npm run dev) &
FRONTEND_PID=$!

trap "kill $BACKEND_PID $FRONTEND_PID 2>/dev/null" EXIT

echo ""
echo "Ready! Open http://localhost:3000 in your browser."
wait

