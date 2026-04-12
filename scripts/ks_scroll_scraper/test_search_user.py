"""
通过 Chrome 搜索页面将自定义 ID 转换为 3x 格式快手 ID。
"""
import sqlite3, os, json, sys, subprocess, time, asyncio, urllib.request

DB_PATH = os.path.join(os.environ["APPDATA"], "com.qingshi.app", "app_data.db")
CDP_PORT = 9333

def read_cookie():
    conn = sqlite3.connect(DB_PATH)
    row = conn.execute(
        "SELECT name, cookies FROM platform_accounts WHERE platform='kuaishou' AND cookies IS NOT NULL ORDER BY updated_at DESC LIMIT 1"
    ).fetchone()
    conn.close()
    return row

def find_chrome():
    for p in [
        os.path.join(os.environ.get("LOCALAPPDATA",""), "Google\\Chrome\\Application\\chrome.exe"),
        "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    ]:
        if os.path.exists(p):
            return p
    return None

async def main():
    search_id = sys.argv[1] if len(sys.argv) > 1 else "xhh52088610000"

    row = read_cookie()
    if not row:
        print("ERR: no kuaishou account"); return
    cookie_str = row[1]

    chrome = find_chrome()
    if not chrome:
        print("ERR: Chrome not found"); return

    user_data = os.path.join(os.environ["TEMP"], "ks_search_test_chrome")
    os.makedirs(user_data, exist_ok=True)
    proc = subprocess.Popen([
        chrome, "--app=about:blank",
        f"--remote-debugging-port={CDP_PORT}",
        f"--user-data-dir={user_data}",
        "--window-size=1200,900",
        "--no-first-run", "--no-default-browser-check", "--disable-extensions",
    ])
    time.sleep(3)

    try:
        import websockets
    except ImportError:
        os.system("pip install websockets -q")
        import websockets

    ws_url = None
    for _ in range(10):
        try:
            resp = urllib.request.urlopen(f"http://127.0.0.1:{CDP_PORT}/json", timeout=2)
            tabs = json.loads(resp.read().decode())
            ws_url = next(t["webSocketDebuggerUrl"] for t in tabs if t["type"] == "page")
            break
        except:
            time.sleep(1)
    if not ws_url:
        print("ERR: CDP connect failed"); proc.kill(); return

    msg_id = 0
    async with websockets.connect(ws_url, max_size=10*1024*1024) as ws:
        async def cmd(method, params=None):
            nonlocal msg_id; msg_id += 1
            await ws.send(json.dumps({"id": msg_id, "method": method, "params": params or {}}))
            while True:
                r = json.loads(await ws.recv())
                if r.get("id") == msg_id:
                    return r

        async def js(expression):
            r = await cmd("Runtime.evaluate", {"expression": expression, "awaitPromise": True})
            return r.get("result", {}).get("result", {}).get("value", "")

        await cmd("Network.enable")
        await cmd("Page.enable")

        cookies = []
        for part in cookie_str.split(";"):
            part = part.strip()
            if "=" in part:
                k, v = part.split("=", 1)
                cookies.append({"name": k.strip(), "value": v.strip(), "domain": ".kuaishou.com", "path": "/"})
        await cmd("Network.setCookies", {"cookies": cookies})
        print(f"[1] Cookies injected ({len(cookies)})")

        # Navigate to search page
        url = f"https://www.kuaishou.com/search/{search_id}?source=SEARCH"
        print(f"[2] Navigate: {url}")
        await cmd("Page.navigate", {"url": url})
        await asyncio.sleep(5)

        # Find and click "用户" tab, then wait
        print("[3] Click user tab...")
        clicked = await js("""(async () => {
            // Find all elements that could be the user tab
            const allEls = document.querySelectorAll('button, div, span, a');
            for (const el of allEls) {
                const text = el.textContent.trim();
                if (text === '用户' && el.offsetParent !== null) {
                    el.click();
                    return 'clicked: ' + el.tagName + ' "' + text + '"';
                }
            }
            return 'not found';
        })()""")
        print(f"    {clicked}")
        await asyncio.sleep(5)

        # Check current page state
        state = await js("""(function() {
            const body = document.body.innerText;
            const hasKuaishouId = body.includes('快手号');
            const hasUserCards = body.includes('关注') && body.includes('订阅');

            // Find all buttons/tabs to see which is active
            const tabs = [];
            document.querySelectorAll('button').forEach(b => {
                const text = b.textContent.trim();
                if (text === '视频' || text === '用户') {
                    const style = window.getComputedStyle(b);
                    const isActive = b.classList.toString();
                    tabs.push({text, class: isActive, color: style.color});
                }
            });

            return JSON.stringify({
                bodyLen: body.length,
                hasKuaishouId,
                hasUserCards,
                tabs,
                bodyFirst500: body.substring(0, 500),
            });
        })()""")
        data = json.loads(state) if state else {}
        print(f"[4] Body: {data.get('bodyLen')} chars, has '快手号': {data.get('hasKuaishouId')}")
        print(f"    Tabs: {json.dumps(data.get('tabs', []), ensure_ascii=False)}")

        if not data.get('hasKuaishouId'):
            # The user tab didn't load. Try clicking again with a different strategy.
            print("[5] User tab not loaded, trying again...")
            await js("""(async () => {
                // Try clicking by finding the second tab button
                const buttons = [...document.querySelectorAll('button')];
                const userBtn = buttons.find(b => b.textContent.trim() === '用户');
                if (userBtn) {
                    userBtn.dispatchEvent(new MouseEvent('click', {bubbles: true}));
                }
            })()""")
            await asyncio.sleep(5)

            state2 = await js("document.body.innerText.includes('快手号')")
            print(f"    After retry, has '快手号': {state2}")

        # Extract results
        result = await js("""(function() {
            const body = document.body.innerText;

            // Extract "快手号：3x..." from user cards
            const idRegex = /快手号[：:]\s*(3x[a-z0-9]+)/gi;
            const ids = [];
            let m;
            while ((m = idRegex.exec(body)) !== null) {
                ids.push(m[1]);
            }

            // Fallback: profile links
            const links = [];
            document.querySelectorAll('a[href*="/profile/3x"]').forEach(a => {
                const match = a.href.match(/\\/profile\\/(3x[a-z0-9]+)/i);
                if (match && !links.includes(match[1])) links.push(match[1]);
            });

            return JSON.stringify({kuaishou_ids: [...new Set(ids)], profile_links: links});
        })()""")

        rdata = json.loads(result) if result else {}
        kid = rdata.get("kuaishou_ids", [])
        plinks = rdata.get("profile_links", [])
        print(f"\n[Result] kuaishou_ids from text: {kid[:5]}")
        print(f"[Result] profile_links: {plinks[:5]}")

        first = kid[0] if kid else (plinks[0] if plinks else None)
        if first:
            print(f"\n{'='*40}")
            print(f"  {search_id} -> {first}")
            print(f"{'='*40}")
        else:
            print("\nFAILED")
            sys.stdout.buffer.write(f"Body: {data.get('bodyFirst500','')}\n".encode("utf-8"))

    proc.kill(); proc.wait()
    print("Done")

asyncio.run(main())
