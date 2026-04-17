#!/usr/bin/env node
/**
 * 资源文件加密脚本
 *
 * 用途：把原始可执行文件（如 ffmpeg.exe / yt-dlp.exe）加密为 .dat 格式，
 *       供 Tauri 应用打包时放到 resources/ 目录，运行时由
 *       src-tauri/src/resource_loader.rs 读取解密。
 *
 * 加密算法：XOR 密钥轮转（与 Rust 端 resource_loader::encrypt_bytes 严格对齐）
 *
 * 文件格式：
 *   [4 bytes magic = "QSR1"]
 *   [4 bytes u32 (little-endian) = payload length]
 *   [N bytes payload = XOR-encrypted raw bytes]
 *
 * 用法示例：
 *   node scripts/encrypt-resource.mjs <input-file> <output.dat> <key-string>
 *
 *   # 常规用法（密钥通过命令行传入）：
 *   node scripts/encrypt-resource.mjs ./src-tauri/resources/ffmpeg.exe \
 *                                      ./src-tauri/resources/media.dat \
 *                                      "my_product_key_2026"
 *
 *   # 从环境变量读密钥（避免命令行历史泄露）：
 *   $env:QSR_KEY="my_product_key_2026"
 *   node scripts/encrypt-resource.mjs ./src-tauri/resources/yt-dlp.exe \
 *                                      ./src-tauri/resources/mcore.dat
 *
 * ⚠️ 重要：密钥必须和 Rust 端调用 `ensure_decrypted(app, name, output_name, KEY)` 时
 *         传入的 KEY 完全一致（按 UTF-8 字节比较），否则解密失败。
 */

import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const args = process.argv.slice(2)
const keyFromEnv = process.env.QSR_KEY || ''

const inputPath = args[0]
const outputPath = args[1]
const keyStr = args[2] || keyFromEnv

function usage(msg) {
  if (msg) console.error(`✗ ${msg}`)
  console.error('用法: node scripts/encrypt-resource.mjs <input> <output.dat> <key-string>')
  console.error('      也可用 QSR_KEY 环境变量传递密钥，省略第三个参数')
  process.exit(1)
}

if (!inputPath || !outputPath) usage('缺少 input 或 output 路径')
if (!keyStr) usage('缺少密钥（通过第三个参数或 QSR_KEY 环境变量传递）')
if (!fs.existsSync(inputPath)) usage(`输入文件不存在: ${inputPath}`)

// 防止误覆盖一个看起来是原始 exe 的文件
if (fs.existsSync(outputPath) && !outputPath.endsWith('.dat')) {
  usage(`安全检查：输出路径 ${outputPath} 不是 .dat 后缀，为避免误覆盖原始文件已中止。\n` +
        `若确实要覆盖，请先手动删除目标文件。`)
}

const MAGIC = Buffer.from('QSR1', 'ascii')  // 4 字节：51 53 52 31
const key = Buffer.from(keyStr, 'utf8')

console.log(`→ 读取: ${inputPath}`)
const raw = fs.readFileSync(inputPath)
console.log(`  原始大小: ${(raw.length / 1024 / 1024).toFixed(2)} MB (${raw.length} bytes)`)

// XOR 轮转加密
console.log(`→ XOR 加密中...`)
const payload = Buffer.alloc(raw.length)
const keyLen = key.length
for (let i = 0; i < raw.length; i++) {
  payload[i] = raw[i] ^ key[i % keyLen]
}

// 组装文件头：magic + LE u32 长度 + payload
const lenBuf = Buffer.alloc(4)
lenBuf.writeUInt32LE(payload.length, 0)
const out = Buffer.concat([MAGIC, lenBuf, payload])

// 原子写入：先写 .tmp 再 rename，避免写入一半被其他进程读到
const outDir = path.dirname(outputPath)
if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true })
const tmpPath = outputPath + '.tmp'
fs.writeFileSync(tmpPath, out)
fs.renameSync(tmpPath, outputPath)

console.log(`✓ 加密完成: ${outputPath}`)
console.log(`  输出大小: ${(out.length / 1024 / 1024).toFixed(2)} MB`)
console.log(`  魔数: QSR1  密钥长度: ${keyLen} 字节`)
console.log('')
console.log('下一步：')
console.log(`  1. git add ${outputPath}`)
console.log(`  2. 从 git 里移除原始 ${path.basename(inputPath)}（不要再追踪原始 exe）`)
console.log(`  3. Rust 端 find_xxx 里调 resource_loader::ensure_decrypted(app,`)
console.log(`     "${path.basename(outputPath)}", "<混淆的运行时名>.exe", KEY)`)
