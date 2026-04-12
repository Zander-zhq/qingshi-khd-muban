# -*- coding: utf-8 -*-
"""方案验证：通过滚动触发抖音自己的分页请求，并对比参数"""
import asyncio
import json
import sys
import io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
from playwright.async_api import async_playwright
from urllib.parse import urlparse, parse_qs

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.connect_over_cdp("http://127.0.0.1:9224")
        ctx = browser.contexts[0]
        page = ctx.pages[0] if ctx.pages else await ctx.new_page()
        print(f"当前页面: {page.url}")

        # 先导航到目标页面
        target = "https://www.douyin.com/user/MS4wLjABAAAATMFkmqt_0RMxUhDFfKMiWtOFJFsqLUAS7KPM_WyFako"
        if 'MS4wLjABAAAATMFkmqt' not in page.url:
            print(f"导航到: {target}")
            await page.goto(target, wait_until="domcontentloaded", timeout=30000)
            await asyncio.sleep(5)

        # 找到所有可滚动的容器
        print("\n=== 分析页面滚动容器 ===")
        containers_info = await page.evaluate("""
            () => {
                const all = document.querySelectorAll('*');
                const scrollable = [];
                for (const el of all) {
                    if (el.scrollHeight > el.clientHeight + 50 && el.clientHeight > 100) {
                        scrollable.push({
                            tag: el.tagName,
                            class: el.className?.substring(0, 120) || '',
                            id: el.id || '',
                            scrollHeight: el.scrollHeight,
                            clientHeight: el.clientHeight,
                            scrollTop: el.scrollTop,
                            overflow: getComputedStyle(el).overflow,
                            overflowY: getComputedStyle(el).overflowY,
                        });
                    }
                }
                return JSON.stringify(scrollable);
            }
        """)
        containers = json.loads(containers_info)
        print(f"  找到 {len(containers)} 个可滚动容器:")
        for i, c in enumerate(containers):
            print(f"  [{i}] <{c['tag']}> class='{c['class'][:80]}' id='{c['id']}' "
                  f"scrollH={c['scrollHeight']} clientH={c['clientHeight']} overflow={c['overflowY']}")

        # 也检查 document.documentElement 和 body
        root_info = await page.evaluate("""
            () => JSON.stringify({
                html: {scrollHeight: document.documentElement.scrollHeight, clientHeight: document.documentElement.clientHeight, scrollTop: document.documentElement.scrollTop},
                body: {scrollHeight: document.body.scrollHeight, clientHeight: document.body.clientHeight, scrollTop: document.body.scrollTop},
            })
        """)
        print(f"  html/body: {root_info}")

        # 监听网络请求
        captured = []
        captured_responses = []

        async def on_request(request):
            if '/aweme/v1/web/aweme/post/' in request.url:
                captured.append({'url': request.url, 'headers': dict(request.headers)})

        async def on_response(response):
            if '/aweme/v1/web/aweme/post/' in response.url:
                try:
                    body = await response.text()
                    body_json = json.loads(body) if body else {}
                except:
                    body_json = {}
                    body = ""
                captured_responses.append({
                    'url': response.url,
                    'status': response.status,
                    'body_length': len(body),
                    'items': len(body_json.get('aweme_list', [])),
                    'has_more': body_json.get('has_more'),
                    'max_cursor': body_json.get('max_cursor'),
                })

        page.on("request", on_request)
        page.on("response", on_response)

        # 多次滚动
        print("\n=== 开始滚动测试 ===")
        for attempt in range(5):
            before = len(captured)
            # 滚动所有可能的容器
            await page.evaluate("""
                () => {
                    window.scrollTo(0, document.body.scrollHeight);
                    document.documentElement.scrollTo(0, document.documentElement.scrollHeight);
                    document.querySelectorAll('*').forEach(el => {
                        if (el.scrollHeight > el.clientHeight + 100 && el.clientHeight > 200) {
                            el.scrollTo(0, el.scrollHeight);
                        }
                    });
                }
            """)
            await asyncio.sleep(3)
            after = len(captured)
            triggered = after - before
            print(f"  第{attempt+1}次滚动: 触发了 {triggered} 个新请求 (累计 {after})")
            if triggered > 0:
                break

        # 如果仍然没有，尝试用鼠标滚轮
        if not captured:
            print("\n=== 尝试鼠标滚轮方案 ===")
            for attempt in range(3):
                await page.mouse.wheel(0, 3000)
                await asyncio.sleep(3)
                if captured:
                    print(f"  鼠标滚轮第{attempt+1}次触发了请求!")
                    break
                print(f"  鼠标滚轮第{attempt+1}次没有触发")

        page.remove_listener("request", on_request)
        page.remove_listener("response", on_response)

        print(f"\n=== 结果: 捕获 {len(captured)} 个请求, {len(captured_responses)} 个响应 ===")

        for i, req in enumerate(captured[:3]):
            print(f"\n--- 请求 {i+1} ---")
            parsed = urlparse(req['url'])
            params = parse_qs(parsed.query)
            all_params = sorted(params.keys())
            print(f"  总参数数量: {len(all_params)}")
            print(f"  所有参数名: {', '.join(all_params)}")
            for p_name in ['X-Bogus', 'a_bogus', 'msToken', 'verifyFp']:
                if p_name in params:
                    val = params[p_name][0]
                    print(f"  ** {p_name}: {val[:60]}... (len={len(val)})" if len(val) > 60 else f"  ** {p_name}: {val}")

        for i, resp in enumerate(captured_responses[:3]):
            print(f"\n--- 响应 {i+1} ---")
            print(f"  HTTP: {resp['status']}, body长度: {resp['body_length']}, 视频数: {resp['items']}")
            print(f"  has_more: {resp['has_more']}, max_cursor: {resp['max_cursor']}")

asyncio.run(main())
