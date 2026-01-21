use std::fs;
use std::path::Path;

use crate::stats::Stats;

pub fn generate_index_html(root: &Path, subpath: &str) -> String {
    let current_path = if subpath.is_empty() {
        root.to_path_buf()
    } else {
        root.join(subpath)
    };

    let entries = match fs::read_dir(&current_path) {
        Ok(entries) => entries,
        Err(_) => return error_html("Cannot read directory"),
    };

    let mut files_html = String::new();
    let mut items: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    items.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in items {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = path.is_dir();

        let (icon, size_str, file_class) = if is_dir {
            ("📁", "-".to_string(), "folder")
        } else {
            let size = path.metadata().map(|m| m.len()).unwrap_or(0);
            (get_file_icon(&name), Stats::format_bytes(size), "file")
        };

        let relative_path = if subpath.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", subpath, name)
        };

        let href = if is_dir {
            format!("/browse/{}", encode_path(&relative_path))
        } else {
            format!("/download/{}", encode_path(&relative_path))
        };

        files_html.push_str(&format!(
            r#"<a href="{}" class="file-item {}">
                <span class="file-icon">{}</span>
                <span class="file-name">{}</span>
                <span class="file-size">{}</span>
            </a>"#,
            href,
            file_class,
            icon,
            html_escape(&name),
            size_str
        ));
    }

    if files_html.is_empty() {
        files_html = r#"<div class="empty">📭 This directory is empty</div>"#.to_string();
    }

    let breadcrumb = generate_breadcrumb(subpath);
    let dir_name = root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Files".to_string());

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>rustserve - {}</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
            min-height: 100vh;
            color: #e0e0e0;
        }}
        
        .container {{
            max-width: 900px;
            margin: 0 auto;
            padding: 40px 20px;
        }}
        
        .header {{
            text-align: center;
            margin-bottom: 40px;
        }}
        
        .header h1 {{
            font-size: 2.5rem;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            margin-bottom: 10px;
        }}
        
        .header .subtitle {{
            color: #888;
            font-size: 1rem;
        }}
        
        .breadcrumb {{
            background: rgba(255, 255, 255, 0.05);
            border-radius: 12px;
            padding: 15px 20px;
            margin-bottom: 20px;
            backdrop-filter: blur(10px);
            border: 1px solid rgba(255, 255, 255, 0.1);
        }}
        
        .breadcrumb a {{
            color: #667eea;
            text-decoration: none;
            transition: color 0.2s;
        }}
        
        .breadcrumb a:hover {{
            color: #764ba2;
        }}
        
        .breadcrumb span {{
            color: #666;
            margin: 0 8px;
        }}
        
        .file-list {{
            background: rgba(255, 255, 255, 0.03);
            border-radius: 16px;
            overflow: hidden;
            backdrop-filter: blur(10px);
            border: 1px solid rgba(255, 255, 255, 0.1);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
        }}
        
        .file-item {{
            display: flex;
            align-items: center;
            padding: 16px 24px;
            text-decoration: none;
            color: #e0e0e0;
            border-bottom: 1px solid rgba(255, 255, 255, 0.05);
            transition: all 0.2s ease;
        }}
        
        .file-item:last-child {{
            border-bottom: none;
        }}
        
        .file-item:hover {{
            background: rgba(102, 126, 234, 0.1);
            transform: translateX(5px);
        }}
        
        .file-item.folder:hover {{
            background: rgba(255, 193, 7, 0.1);
        }}
        
        .file-icon {{
            font-size: 1.5rem;
            margin-right: 16px;
            width: 32px;
            text-align: center;
        }}
        
        .file-name {{
            flex: 1;
            font-weight: 500;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }}
        
        .file-size {{
            color: #888;
            font-size: 0.9rem;
            font-family: 'Monaco', 'Consolas', monospace;
        }}
        
        .empty {{
            text-align: center;
            padding: 60px 20px;
            color: #666;
            font-size: 1.2rem;
        }}
        
        .footer {{
            text-align: center;
            margin-top: 40px;
            color: #555;
            font-size: 0.85rem;
        }}
        
        .footer a {{
            color: #667eea;
            text-decoration: none;
        }}
        
        @media (max-width: 600px) {{
            .container {{
                padding: 20px 15px;
            }}
            
            .header h1 {{
                font-size: 1.8rem;
            }}
            
            .file-item {{
                padding: 12px 16px;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 rustserve</h1>
            <p class="subtitle">Serving files from <strong>{}</strong></p>
        </div>
        
        <div class="breadcrumb">
            {}
        </div>
        
        <div class="file-list">
            {}
        </div>
        
        <div class="footer">
            Powered by <a href="https://github.com/rustserve">rustserve</a>
        </div>
    </div>
</body>
</html>"#,
        html_escape(&dir_name),
        html_escape(&dir_name),
        breadcrumb,
        files_html
    )
}

fn generate_breadcrumb(subpath: &str) -> String {
    let mut html = r#"<a href="/">📂 Home</a>"#.to_string();

    if !subpath.is_empty() {
        let parts: Vec<&str> = subpath.split('/').collect();
        let mut accumulated = String::new();

        for part in parts {
            if !part.is_empty() {
                accumulated = if accumulated.is_empty() {
                    part.to_string()
                } else {
                    format!("{}/{}", accumulated, part)
                };
                html.push_str(&format!(
                    r#" <span>/</span> <a href="/browse/{}">{}</a>"#,
                    encode_path(&accumulated),
                    html_escape(part)
                ));
            }
        }
    }

    html
}

fn get_file_icon(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        // Images
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "ico" => "🖼️",
        // Videos
        "mp4" | "webm" | "avi" | "mov" | "mkv" | "flv" => "🎬",
        // Audio
        "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a" => "🎵",
        // Documents
        "pdf" => "📕",
        "doc" | "docx" => "📘",
        "xls" | "xlsx" => "📗",
        "ppt" | "pptx" => "📙",
        "txt" | "md" | "rtf" => "📄",
        // Code
        "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "h" => "💻",
        "html" | "css" | "scss" | "sass" => "🌐",
        "json" | "yaml" | "yml" | "toml" | "xml" => "⚙️",
        // Archives
        "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" => "📦",
        // Executables
        "exe" | "msi" | "dmg" | "app" | "deb" | "rpm" => "⚡",
        // Default
        _ => "📄",
    }
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn encode_path(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace('#', "%23")
        .replace('?', "%3F")
}

pub fn error_html(message: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Error - rustserve</title>
    <style>
        body {{
            font-family: system-ui, sans-serif;
            background: #1a1a2e;
            color: #e0e0e0;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
        }}
        .error {{
            text-align: center;
            padding: 40px;
            background: rgba(255, 0, 0, 0.1);
            border-radius: 16px;
            border: 1px solid rgba(255, 0, 0, 0.3);
        }}
        .error h1 {{ color: #ff6b6b; }}
    </style>
</head>
<body>
    <div class="error">
        <h1>⚠️ Error</h1>
        <p>{}</p>
        <p><a href="/" style="color: #667eea;">← Back to home</a></p>
    </div>
</body>
</html>"#,
        html_escape(message)
    )
}
