"""测试抖音主页 fetch() 翻页，检查为什么只返回第一页"""
import asyncio
import json
from playwright.async_api import async_playwright

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.connect_over_cdp("http://127.0.0.1:9224")
        ctx = browser.contexts[0]
        page = ctx.pages[0] if ctx.pages else await ctx.new_page()

        print(f"当前页面: {page.url}")

        # 从当前 URL 提取 sec_user_id
        import re
        m = re.search(r'user/([^?/]+)', page.url)
        if not m:
            print("当前页面不在抖音用户主页，请先打开一个抖音主页")
            return
        sec_uid = m.group(1)
        print(f"sec_user_id: {sec_uid}")

        # 第一次 fetch
        print("\n=== 第1页 fetch (max_cursor=0) ===")
        result1 = await page.evaluate(f"""
            async () => {{
                try {{
                    const url = '/aweme/v1/web/aweme/post/?sec_user_id={sec_uid}&max_cursor=0&count=18&device_platform=webapp&aid=6383';
                    console.log('[TEST] fetch URL:', url);
                    const r = await fetch(url);
                    const text = await r.text();
                    return JSON.stringify({{status: r.status, headers: Object.fromEntries(r.headers.entries()), bodyLength: text.length, body: text}});
                }} catch(e) {{
                    return JSON.stringify({{error: e.message, stack: e.stack}});
                }}
            }}
        """)
        r1 = json.loads(result1)
        print(f"  HTTP status: {r1.get('status')}")
        print(f"  body长度: {r1.get('bodyLength')}")

        if 'error' in r1:
            print(f"  错误: {r1['error']}")
            return

        try:
            body1 = json.loads(r1.get('body', '{}'))
        except:
            print(f"  body 不是 JSON，前500字符: {r1.get('body', '')[:500]}")
            return

        status_code = body1.get('status_code')
        has_more = body1.get('has_more')
        max_cursor = body1.get('max_cursor')
        aweme_list = body1.get('aweme_list', [])
        print(f"  status_code (API): {status_code}")
        print(f"  has_more: {has_more} (type: {type(has_more).__name__})")
        print(f"  max_cursor: {max_cursor} (type: {type(max_cursor).__name__})")
        print(f"  aweme_list 数量: {len(aweme_list)}")
        if aweme_list:
            print(f"  第一条: id={aweme_list[0].get('aweme_id')}, desc={aweme_list[0].get('desc', '')[:50]}")

        # 关键检查：Rust 代码里的判断条件
        cursor_str = str(max_cursor) if max_cursor is not None else ""
        print(f"\n  --- Rust 判断条件模拟 ---")
        print(f"  max_cursor 转字符串: '{cursor_str}'")
        print(f"  cursor_str.is_empty(): {cursor_str == ''}")
        print(f"  cursor_str == '0': {cursor_str == '0'}")
        print(f"  has_more 转 i64==1: {has_more == 1}")
        print(f"  has_more 转 bool: {has_more is True}")

        # 如果 has_more 和 cursor 都正常，继续翻页
        total_videos = len(aweme_list)

        if not has_more and has_more != 1:
            print(f"\n!!! 第一页就说 has_more={has_more}，不会继续翻页了 !!!")
            return

        if cursor_str == '' or cursor_str == '0':
            print(f"\n!!! max_cursor='{cursor_str}'，Rust 代码会认为没有更多数据退出循环 !!!")
            print(f"但 has_more={has_more}，这说明 cursor 解析有问题")

            # 检查原始 JSON 中 max_cursor 的更多细节
            raw_cursor = body1.get('max_cursor')
            print(f"\n  原始 max_cursor 值: {repr(raw_cursor)}")
            print(f"  原始 JSON 中是否有 'cursor' 字段: {'cursor' in body1}")
            if 'cursor' in body1:
                print(f"  cursor 值: {repr(body1['cursor'])}")

            # 打印 body1 中所有包含 cursor 的 key
            print(f"\n  JSON 根级别所有 key:")
            for k in sorted(body1.keys()):
                v = body1[k]
                if isinstance(v, (str, int, float, bool, type(None))):
                    print(f"    {k}: {repr(v)}")
                else:
                    print(f"    {k}: ({type(v).__name__}, len={len(v) if hasattr(v, '__len__') else '?'})")
            return

        # 翻页测试
        current_cursor = cursor_str
        for page_num in range(2, 10):
            print(f"\n=== 第{page_num}页 fetch (max_cursor={current_cursor}) ===")
            await asyncio.sleep(1.5)

            result = await page.evaluate(f"""
                async () => {{
                    try {{
                        const r = await fetch('/aweme/v1/web/aweme/post/?sec_user_id={sec_uid}&max_cursor={current_cursor}&count=18&device_platform=webapp&aid=6383');
                        const text = await r.text();
                        return JSON.stringify({{status: r.status, bodyLength: text.length, body: text}});
                    }} catch(e) {{
                        return JSON.stringify({{error: e.message}});
                    }}
                }}
            """)
            r = json.loads(result)
            print(f"  HTTP status: {r.get('status')}")
            print(f"  body长度: {r.get('bodyLength')}")

            if 'error' in r:
                print(f"  错误: {r['error']}")
                break

            try:
                body = json.loads(r.get('body', '{}'))
            except:
                print(f"  body 不是 JSON: {r.get('body', '')[:500]}")
                break

            status_code = body.get('status_code')
            has_more = body.get('has_more')
            max_cursor = body.get('max_cursor')
            aweme_list = body.get('aweme_list', [])
            total_videos += len(aweme_list)
            print(f"  status_code: {status_code}")
            print(f"  has_more: {has_more}")
            print(f"  max_cursor: {max_cursor}")
            print(f"  新增视频: {len(aweme_list)}, 累计: {total_videos}")

            cursor_str = str(max_cursor) if max_cursor is not None else ""
            if not has_more and has_more != 1:
                print(f"\n  has_more={has_more}，翻页结束")
                break
            if cursor_str == '' or cursor_str == '0':
                print(f"\n  max_cursor='{cursor_str}'，翻页结束")
                break
            current_cursor = cursor_str

        print(f"\n=== 测试完成，共获取 {total_videos} 个视频 ===")

asyncio.run(main())
