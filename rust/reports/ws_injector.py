"""WS 负载注入器：连接本地 WS 并持续发送 sync 消息，用于 soak 测试。"""
import asyncio
import json
import sys
import time
import websockets

async def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8767
    pin = sys.argv[2] if len(sys.argv) > 2 else "1234"
    duration = int(sys.argv[3]) if len(sys.argv) > 3 else 120
    url = f"ws://127.0.0.1:{port}"
    try:
        async with websockets.connect(url, open_timeout=10) as ws:
            auth = json.dumps({"pin": pin, "type": "send", "text": "soak-init"})
            await ws.send(auth)
            await ws.recv()
            n = 0
            deadline = time.time() + duration
            while time.time() < deadline:
                msg = json.dumps({"pin": pin, "type": "sync", "text": f"soak-{n}"})
                await ws.send(msg)
                try:
                    await asyncio.wait_for(ws.recv(), timeout=2.0)
                except asyncio.TimeoutError:
                    pass
                n += 1
                await asyncio.sleep(2.0)
            print(f"injector: sent {n} messages in {duration}s")
    except Exception as e:
        print(f"injector error: {e}")

asyncio.run(main())
