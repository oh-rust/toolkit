use clap::{ArgAction, Parser};
use colored::*;
use std::io::Write;
use std::io::{self, BufRead};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 是否格式化 json 字符串
    #[arg(short, long,default_value_t = true,action = ArgAction::Set)]
    json: bool,

    /// 是否开启反转义 (\\n -> \n, \\u -> unicode, \" -> ")
    #[arg(short, long, default_value_t = true,action = ArgAction::Set)]
    unquote: bool,

    /// 是否显示行号
    #[arg(short, long,default_value_t = true,action = ArgAction::Set)]
    no: bool,

    /// 是否对文本中部分内容添加颜色（高亮显示）
    #[arg(short, long,default_value_t = true,action = ArgAction::Set)]
    highlight: bool,
}

fn main() {
    let args = Args::parse();

    let stdin = io::stdin();
    let mut line_no = 0;
    for line in stdin.lock().lines() {
        line_no += 1;
        match line {
            Ok(content) => {
                let mut output = content.trim_end().to_string();
                if args.json {
                    output = format_json(&output);
                }
                if args.unquote {
                    output = unquote(&output);
                }
                let mut line_no_str = "".to_string();
                if args.no {
                    let num_str = format!("{:04}", line_no);
                    line_no_str = format!("{}{}  ", num_str.cyan(), "|".bright_black());
                }

                if args.highlight {
                    output = highlight(&output);
                }

                let _ = writeln!(io::stdout(), "{} {}", line_no_str, output);
            }
            Err(error) => {
                eprintln!("read stdin error: {}", error);
                break;
            }
        }
    }
}

// fn escape_all_control(s: &str) -> String {
//     s.chars()
//         .map(|c| {
//              if c.is_control() {
//                 format!("\\x{:02x}", c as u32)
//             } else {
//                 c.to_string()
//             }
//         })
//         .collect()
// }

/// 采用正序扫描与括号计数机制，当识别到 { 后开始统计嵌套深度，仅在括号完全平衡（计数归零）时触发 JSON 解析。
/// 若解析失败（如括号出现在字符串常量内），算法将容错并继续向后搜索，直到定位到合法的最外层 JSON 对象。
/// 成功解析后，利用 serde_json 执行美化缩进重写并跳过已处理区域，确保非 JSON 文本原样保留且嵌套结构清晰易读。
use simd_json::{self, OwnedValue};
use std::io::Cursor;

fn format_json(s: &str) -> String {
    if s.len() < 5 {
        return s.to_string();
    }
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' && (i + 1) < chars.len() && chars[i + 1] == '"' {
            let start_index = i;
            let mut brace_count = 0;
            let mut found_and_parsed = false;

            let mut has_brace = false; // 是否完成首次 {} 匹配

            // 从当前 '{' 开始向后扫描
            for j in i..chars.len() {
                if chars[j] == '{' {
                    brace_count += 1;
                } else if chars[j] == '}' {
                    brace_count -= 1;
                }

                if brace_count == 0 {
                    has_brace = true;
                }

                // 只有当括号平衡时，才尝试解析
                // 当字符串内容中包含 { 或者 } 时，会导致计数不正确，这时一直往后查找尝试
                if brace_count == 0 || has_brace {
                    let potential_json: String = chars[start_index..=j].iter().collect();
                    let reader = Cursor::new(potential_json.as_bytes());
                    // 尝试解析为 JSON 对象
                    if let Ok(value) = simd_json::from_reader::<_, OwnedValue>(reader) {
                        if let Ok(pretty_json) = simd_json::to_string_pretty(&value) {
                            result.push_str(&pretty_json);
                            i = j + 1; // 成功解析，跳过整个区域
                            found_and_parsed = true;
                            break;
                        }
                    }
                }
            }

            if found_and_parsed {
                continue;
            }
        } // end if

        // 如果不是 '{'，或者该 '{' 及其后续内容无法解析为 JSON，则原样移动
        result.push(chars[i]);
        i += 1;
    }

    result
}

/// 使用 colored 实现高亮：
///  1. 文本内容中以 # 开头，或者 trim 后以 # 开头的行，添加绿色
/// 2. 传入参数 s 包含换行符
fn highlight(s: &str) -> String {
    s.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                // 统计连续 # 的数量
                let count = trimmed.chars().take_while(|&c| c == '#').count();
                if count == 1 {
                    line.green().to_string()
                } else {
                    line.cyan().to_string()
                }
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 将字符串中的  \\n --> \n, \\uxxx -> UTF-8 字符
fn unquote(s: &str) -> String {
    let mut res = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                // 处理 \n
                Some('n') => res.push('\n'),
                // 处理 \"
                Some('"') => res.push('"'),

                // 处理 \uXXXX
                Some('u') => {
                    let mut hex = String::new();
                    for _ in 0..4 {
                        if let Some(&h) = chars.peek() {
                            if h.is_ascii_hexdigit() {
                                hex.push(chars.next().unwrap());
                                continue;
                            }
                        }
                        break;
                    }

                    if hex.len() == 4 {
                        if let Some(unicode_char) = u32::from_str_radix(&hex, 16).ok().and_then(std::char::from_u32) {
                            res.push(unicode_char);
                        } else {
                            // 解析 Unicode 失败，原样返回
                            return s.to_string();
                        }
                    } else {
                        // 长度不足 4 位，原样返回
                        return s.to_string();
                    }
                }

                // 其他转义或末尾只有 \ 的情况
                Some(other) => {
                    res.push('\\');
                    res.push(other);
                }
                None => res.push('\\'),
            }
        } else {
            res.push(c);
        }
    }

    res
}
