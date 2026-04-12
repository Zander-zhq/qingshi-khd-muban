"""检查抖音 fetch() 的问题：对比页面自动请求和手动 fetch 的区别"""
import asyncio
import json
from playwright.async_api import async_playwright

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.connect_over_cdp("http://127.0.0.1:9224")
        ctx = browser.contexts[0]
        page = ctx.pages[0] if ctx.pages else await ctx.new_page()
        print(f"当前页面: {page.url}")

        # 方法1: 监听网络请求，看页面滚动时实际发出的 post API 请求格式
        print("\n=== 方法1：捕获页面自动发出的请求 ===")
        captured_urls = []

        async def on_response(response):
            url = response.url
            if '/aweme/v1/web/aweme/post/' in url:
                status = response.status
                try:
                    body = await response.text()
                    body_json = json.loads(body) if body else {}
                    items = len(body_json.get('aweme_list', []))
                    has_more = body_json.get('has_more')
                    max_cursor = body_json.get('max_cursor')
                except:
                    body_json = {}
                    items = 0
                    has_more = None
                    max_cursor = None

                captured_urls.append(url)
                print(f"\n  [自动请求] status={status}, items={items}, has_more={has_more}, max_cursor={max_cursor}")
                print(f"  URL长度: {len(url)}")
                # 提取关键参数
                from urllib.parse import urlparse, parse_qs
                parsed = urlparse(url)
                params = parse_qs(parsed.query)
                important_params = ['sec_user_id', 'max_cursor', 'count', 'X-Bogus', 'a_bogus', 'msToken', 'verifyFp']
                for param in important_params:
                    if param in params:
                        val = params[param][0]
                        if len(val) > 50:
                            print(f"  {param}: {val[:50]}... (len={len(val)})")
                        else:
                            print(f"  {param}: {val}")
                    else:
                        print(f"  {param}: [缺失]")

                # 列出所有参数名
                all_params = sorted(params.keys())
                print(f"  所有参数: {', '.join(all_params)}")

        page.on("response", on_response)

        # 滚动页面触发自动请求
        print("\n滚动页面触发请求...")
        # 找到正确的滚动容器
        scroll_result = await page.evaluate("""
            () => {
                // 尝试各种可能的滚动容器
                const containers = [
                    document.querySelector('.route-scroll-container'),
                    document.querySelector('[class*="scroll"]'),
                    document.querySelector('main'),
                    document.documentElement,
                    document.body,
                ];
                for (const c of containers) {
                    if (c) {
                        const info = {
                            tag: c.tagName,
                            class: c.className?.substring(0, 100),
                            scrollHeight: c.scrollHeight,
                            clientHeight: c.clientHeight,
                            scrollTop: c.scrollTop,
                        };
                        if (c.scrollHeight > c.clientHeight + 100) {
                            c.scrollTo(0, c.scrollHeight);
                            return JSON.stringify({scrolled: true, container: info});
                        }
                    }
                }
                // fallback
                window.scrollTo(0, document.body.scrollHeight);
                return JSON.stringify({scrolled: true, container: 'fallback-body'});
            }
        """)
        print(f"  滚动结果: {scroll_result}")
        await asyncio.sleep(5)

        # 再滚动一次
        await page.evaluate("""
            () => {
                const c = document.querySelector('.route-scroll-container') || document.documentElement;
                c.scrollTo(0, c.scrollHeight);
            }
        """)
        await asyncio.sleep(5)

        page.remove_listener("response", on_response)

        print(f"\n捕获到 {len(captured_urls)} 个 post API 请求")

        if captured_urls:
            # 方法2: 用捕获到的完整 URL 来 fetch，看看是否有数据
            print("\n=== 方法2：用捕获到的完整 URL 来 fetch ===")
            test_url = captured_urls[-1]
            result = await page.evaluate(f"""
                async () => {{
                    try {{
                        const r = await fetch("{test_url}");
                        const text = await r.text();
                        return JSON.stringify({{status: r.status, bodyLength: text.length}});
                    }} catch(e) {{
                        return JSON.stringify({{error: e.message}});
                    }}
                }}
            """)
            print(f"  用完整URL fetch: {result}")

        # 方法3: 检查是否有 service worker 或请求拦截
        print("\n=== 方法3：检查 XMLHttpRequest vs fetch ===")
        result_xhr = await page.evaluate("""
            () => {
                return new Promise((resolve) => {
                    const xhr = new XMLHttpRequest();
                    xhr.open('GET', '/aweme/v1/web/aweme/post/?sec_user_id=MS4wLjABAAAATMFkmqt_0RMxUhDFfKMiWtOFJFsqLUAS7KPM_WyFako&max_cursor=0&count=18&device_platform=webapp&aid=6383');
                    xhr.onload = () => {
                        resolve(JSON.stringify({status: xhr.status, bodyLength: xhr.responseText.length, bodyPreview: xhr.responseText.substring(0, 200)}));
                    };
                    xhr.onerror = (e) => {
                        resolve(JSON.stringify({error: 'XHR error'}));
                    };
                    xhr.send();
                });
            }
        """)
        print(f"  XHR 结果: {result_xhr}")

        print("\n=== 测试完成 ===")

asyncio.run(main())
