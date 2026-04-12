import sqlite3, os, json, urllib.request

db = os.path.join(os.environ['APPDATA'], 'com.qingshi.app', 'app_data.db')
conn = sqlite3.connect(db)
cookie = conn.execute(
    "SELECT cookies FROM platform_accounts WHERE platform=? LIMIT 1",
    ('kuaishou',)
).fetchone()[0]
conn.close()

test_ids = ['ziqishuo', 'xhh52088610000', 'C68686886C', 'Q3500463680', 'QQ_2627174']

print("=== live_api with full params ===")
for uid in test_ids:
    url = f'https://live.kuaishou.com/live_api/profile/public?principalId={uid}&count=12&privacy=public&caver=2&hasMore=true'
    req = urllib.request.Request(url, headers={
        'Cookie': cookie,
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
        'Referer': f'https://live.kuaishou.com/profile/{uid}',
    })
    try:
        resp = json.loads(urllib.request.urlopen(req).read())
        result = resp.get('data', {}).get('result')
        items = resp.get('data', {}).get('list', [])
        if items:
            first = items[0]
            pid = first.get('principalId', first.get('userId', ''))
            name = first.get('userName', first.get('name', ''))
            print(f"  {uid:20s} -> result={result}, items={len(items)}, principalId={pid}, name={name}")
        else:
            print(f"  {uid:20s} -> result={result}, items=0")
    except Exception as e:
        print(f"  {uid:20s} -> ERROR: {e}")
