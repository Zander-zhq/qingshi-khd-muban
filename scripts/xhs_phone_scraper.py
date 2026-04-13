"""
小红书手机端作者主页抓取脚本
通过 ADB UI dump + 自动滚动，逐屏提取作者主页上的所有笔记数据
"""
import subprocess
import xml.etree.ElementTree as ET
import re
import json
import time
import sys
import os

sys.stdout.reconfigure(encoding='utf-8')
sys.stderr.reconfigure(encoding='utf-8')

DEVICE_ID = "26231JEGR13588"
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "xhs_captures")
os.makedirs(OUTPUT_DIR, exist_ok=True)

def adb_cmd(cmd):
    full = f'adb -s {DEVICE_ID} {cmd}'
    result = subprocess.run(full, shell=True, capture_output=True, text=True, encoding='utf-8', errors='ignore')
    return result.stdout + result.stderr

def dump_ui():
    adb_cmd('shell uiautomator dump /sdcard/ui_dump.xml')
    xml_str = adb_cmd('shell cat /sdcard/ui_dump.xml')
    return xml_str

def scroll_down():
    adb_cmd('shell input swipe 540 1800 540 600 500')
    time.sleep(1.5)

def parse_notes_from_xml(xml_str):
    """从 UI XML 中提取笔记信息，基于 content-desc 格式：
    '视频,标题,来自作者名,赞数赞，' 或 '笔记,标题,来自作者名,赞数赞'
    """
    notes = []
    try:
        root = ET.fromstring(xml_str)
    except ET.ParseError:
        return notes

    pattern = re.compile(r'^(视频|笔记)[,，]\s*(.+?)[,，]\s*来自(.+?)[,，]\s*(\d+)赞')

    for node in root.iter('node'):
        desc = node.get('content-desc', '')
        if not desc:
            continue
        m = pattern.match(desc)
        if m:
            note_type = m.group(1)
            title = m.group(2).strip()
            author = m.group(3).strip()
            likes = int(m.group(4))
            notes.append({
                'type': note_type,
                'title': title,
                'author': author,
                'likes': likes,
                'raw_desc': desc,
            })
    return notes

def parse_author_info(xml_str):
    """提取作者基本信息"""
    info = {}
    try:
        root = ET.fromstring(xml_str)
    except ET.ParseError:
        return info

    for node in root.iter('node'):
        text = node.get('text', '')
        desc = node.get('content-desc', '')
        if '粉丝' in desc:
            m = re.search(r'([\d.]+万?)粉丝', desc)
            if m:
                info['followers'] = m.group(1)
        if '获赞与收藏' in desc:
            m = re.search(r'([\d.]+万?)获赞与收藏', desc)
            if m:
                info['likes_and_favs'] = m.group(1)
        if '小红书号' in text:
            info['xhs_id'] = text.replace('小红书号：', '').strip()
        if 'IP：' in text:
            info['ip_location'] = text.replace('IP：', '').strip()

    return info


def main():
    print("=" * 60)
    print("  小红书作者主页 ADB 抓取")
    print("=" * 60)

    xml_str = dump_ui()
    author_info = parse_author_info(xml_str)
    print(f"\n📋 作者信息: {json.dumps(author_info, ensure_ascii=False, indent=2)}")

    all_notes = []
    seen_titles = set()
    no_new_count = 0
    max_no_new = 5
    round_num = 0

    while True:
        round_num += 1
        xml_str = dump_ui()
        notes = parse_notes_from_xml(xml_str)

        new_count = 0
        for note in notes:
            key = f"{note['title']}_{note['likes']}"
            if key not in seen_titles:
                seen_titles.add(key)
                all_notes.append(note)
                new_count += 1
                print(f"  [{len(all_notes):3d}] {note['type']} | {note['title'][:40]:<40s} | ❤️ {note['likes']}")

        if new_count == 0:
            no_new_count += 1
            print(f"  ⏳ 第{round_num}轮: 无新增 ({no_new_count}/{max_no_new})")
            if no_new_count >= max_no_new:
                print(f"\n✅ 连续{max_no_new}轮无新增，抓取结束")
                break
        else:
            no_new_count = 0
            print(f"  📥 第{round_num}轮: 新增 {new_count} 条，累计 {len(all_notes)} 条")

        scroll_down()

    video_count = sum(1 for n in all_notes if n['type'] == '视频')
    note_count = sum(1 for n in all_notes if n['type'] == '笔记')

    print(f"\n{'=' * 60}")
    print(f"  抓取完成!")
    print(f"  总计: {len(all_notes)} 条 (视频 {video_count} + 笔记 {note_count})")
    print(f"{'=' * 60}")

    output = {
        'author_info': author_info,
        'total': len(all_notes),
        'video_count': video_count,
        'note_count': note_count,
        'notes': all_notes,
    }

    author_name = all_notes[0]['author'] if all_notes else 'unknown'
    filename = f"xhs_{author_name}_{int(time.time())}.json"
    filepath = os.path.join(OUTPUT_DIR, filename)
    with open(filepath, 'w', encoding='utf-8') as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    print(f"\n💾 数据已保存到: {filepath}")

    return output


if __name__ == '__main__':
    main()
