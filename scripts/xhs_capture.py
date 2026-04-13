"""
小红书 API 抓包脚本 - 用于 mitmdump
捕获小红书 App 的 API 请求和响应，重点关注用户主页和视频数据
"""
import json
import os
import time
from mitmproxy import http

LOG_DIR = os.path.join(os.path.dirname(__file__), "xhs_captures")
os.makedirs(LOG_DIR, exist_ok=True)

INTERESTING_KEYWORDS = [
    "user/posted",
    "user/profile",
    "user_posted",
    "user_profile",
    "feed",
    "note",
    "video",
    "homefeed",
    "v1/note",
    "v2/note",
    "sns/v1",
    "sns/v2",
    "api/sns",
]

capture_count = 0

def response(flow: http.HTTPFlow) -> None:
    global capture_count
    url = flow.request.pretty_url
    host = flow.request.host

    if "xiaohongshu" not in host and "xhscdn" not in host and "xhs" not in host:
        return

    is_interesting = any(kw in url.lower() for kw in INTERESTING_KEYWORDS)

    status = flow.response.status_code if flow.response else "?"
    content_type = flow.response.headers.get("content-type", "") if flow.response else ""

    print(f"\n{'='*80}")
    print(f"[XHS] {flow.request.method} {url}")
    print(f"  Status: {status}  Content-Type: {content_type}")

    if is_interesting and flow.response and flow.response.content:
        capture_count += 1
        body = flow.response.content.decode("utf-8", errors="ignore")
        print(f"  >>> 关键接口! 响应大小: {len(body)} bytes")

        try:
            data = json.loads(body)
            filename = f"capture_{capture_count}_{int(time.time())}.json"
            filepath = os.path.join(LOG_DIR, filename)
            with open(filepath, "w", encoding="utf-8") as f:
                json.dump({
                    "url": url,
                    "method": flow.request.method,
                    "status": status,
                    "request_headers": dict(flow.request.headers),
                    "response_data": data,
                }, f, ensure_ascii=False, indent=2)
            print(f"  >>> 已保存到: {filepath}")

            if "data" in data:
                d = data["data"]
                if "notes" in d:
                    notes = d["notes"]
                    print(f"  >>> 包含 {len(notes)} 条笔记数据!")
                    for i, note in enumerate(notes[:3]):
                        nid = note.get("note_id", "?")
                        title = note.get("display_title", note.get("title", "?"))
                        ntype = note.get("type", "?")
                        print(f"      [{i}] id={nid} type={ntype} title={title[:30]}")
                if "user" in d:
                    user = d["user"]
                    print(f"  >>> 用户: {user.get('nickname', '?')} (id={user.get('user_id', '?')})")
        except json.JSONDecodeError:
            filename = f"capture_{capture_count}_{int(time.time())}.txt"
            filepath = os.path.join(LOG_DIR, filename)
            with open(filepath, "w", encoding="utf-8") as f:
                f.write(f"URL: {url}\n\n{body[:5000]}")
            print(f"  >>> 非JSON响应, 已保存前5000字符到: {filepath}")
