#![allow(dead_code)]

//! File Icon Theme for SFTP File Manager
//! Provides icons for different file types based on extensions

#[derive(Clone, Debug)]
pub struct FileIconTheme;

impl Default for FileIconTheme {
    fn default() -> Self {
        Self
    }
}

impl FileIconTheme {
    /// Get icon for a file based on name and whether it's a directory
    pub fn get_icon(&self, name: &str, is_dir: bool) -> &'static str {
        if is_dir {
            return "📁";
        }

        let ext = name.split('.').next_back().unwrap_or("").to_lowercase();

        match ext.as_str() {
            // Code files
            "rs" => "🦀",
            "js" | "jsx" | "ts" | "tsx" | "mjs" => "📜",
            "py" | "pyc" | "pyo" => "🐍",
            "java" | "class" | "jar" => "☕",
            "cpp" | "cc" | "cxx" | "hpp" | "h" | "c" => "⚙️",
            "go" => "🐹",
            "rb" | "erb" => "💎",
            "php" => "🐘",
            "swift" => "🦉",
            "kt" | "kts" => "🎯",
            "scala" => "🔺",
            "r" | "rmd" => "📊",
            "m" | "mm" => "🍎",
            "cs" => "🔷",
            "fs" | "fsx" => "🔷",
            "hs" | "lhs" => "λ",
            "erl" | "hrl" => "☎️",
            "ex" | "exs" => "💧",
            "clj" | "cljs" | "cljc" => "◎",
            "jl" => "🔬",
            "lua" => "🌙",
            "pl" | "pm" => "🐪",
            "sh" | "bash" | "zsh" | "fish" | "ksh" => "🐚",
            "ps1" | "psm1" | "psd1" => "🪟",
            "vim" | "nvim" => "📝",

            // Web files
            "html" | "htm" | "xhtml" => "🌐",
            "css" | "scss" | "sass" | "less" | "styl" => "🎨",
            "json" | "jsonc" => "📋",
            "xml" | "xsd" | "xsl" | "xslt" | "svg" => "📄",
            "yaml" | "yml" => "📄",
            "toml" => "⚙️",
            "ini" | "cfg" | "conf" | "config" => "⚙️",

            // Document files
            "md" | "markdown" | "mdx" => "📝",
            "txt" | "text" | "rtf" => "📄",
            "pdf" => "📕",
            "doc" | "docx" => "📘",
            "xls" | "xlsx" | "xlsm" => "📗",
            "ppt" | "pptx" => "📙",
            "odt" | "ods" | "odp" => "📄",
            "tex" | "latex" => "📜",

            // Image files
            "jpg" | "jpeg" | "jpe" => "🖼️",
            "png" => "🖼️",
            "gif" => "🎭",
            "bmp" => "🖼️",
            "tiff" | "tif" => "🖼️",
            "webp" => "🖼️",
            "ico" => "🎯",
            "heic" | "heif" => "📷",
            "raw" | "cr2" | "nef" | "arw" => "📷",
            "psd" | "ai" | "sketch" => "🎨",

            // Audio files
            "mp3" => "🎵",
            "wav" | "wave" => "🔊",
            "flac" => "🎼",
            "aac" | "m4a" => "🎵",
            "ogg" | "oga" => "🎵",
            "wma" => "🎵",
            "aiff" | "au" => "🔊",

            // Video files
            "mp4" | "m4v" => "🎬",
            "avi" => "🎬",
            "mkv" => "🎬",
            "mov" | "qt" => "🎬",
            "wmv" => "🎬",
            "flv" | "f4v" => "🎬",
            "webm" => "🎬",
            "mpg" | "mpeg" | "mp2" => "🎬",
            "3gp" => "📱",

            // Archive files
            "zip" => "🗜️",
            "tar" => "📦",
            "gz" | "gzip" => "🗜️",
            "bz2" | "bzip2" => "🗜️",
            "xz" => "🗜️",
            "7z" => "🗜️",
            "rar" => "🗜️",
            "iso" | "img" => "💿",
            "dmg" => "💿",

            // Database files
            "sql" => "🗄️",
            "db" | "sqlite" | "sqlite3" => "🗄️",
            "mdb" | "accdb" => "🗄️",

            // Executable files
            "exe" | "dll" | "sys" => "⚙️",
            "msi" => "📦",
            "app" => "🚀",
            "deb" | "rpm" | "pkg" | "apk" => "📦",
            "bin" => "⚙️",
            "so" | "dylib" => "⚙️",

            // Log files
            "log" => "📋",

            // Lock files
            "lock" => "🔒",

            // Git files
            "gitignore" | "gitattributes" | "gitmodules" => "🔀",

            // Docker files
            "dockerfile" | "dockerignore" => "🐳",

            // License files
            "license" | "licence" | "copying" | "notice" => "⚖️",

            // README files
            "readme" => "📖",

            // Certificate files
            "pem" | "crt" | "cer" | "key" | "pub" | "ppk" => "🔐",
            "p12" | "pfx" => "🔐",

            // Font files
            "ttf" | "otf" | "woff" | "woff2" | "eot" => "🔤",

            // 3D files
            "obj" | "fbx" | "stl" | "3ds" | "blend" | "dae" => "🎲",

            _ => "📄",
        }
    }

    /// Get icon with color indication for file type category
    pub fn get_icon_with_category(&self, name: &str, is_dir: bool) -> (&'static str, &'static str) {
        let icon = self.get_icon(name, is_dir);

        if is_dir {
            return (icon, "folder");
        }

        let ext = name.split('.').next_back().unwrap_or("").to_lowercase();

        let category = match ext.as_str() {
            "rs" | "js" | "jsx" | "ts" | "tsx" | "py" | "java" | "cpp" | "c" | "h" | "hpp"
            | "go" | "rb" | "php" | "swift" | "kt" | "scala" | "r" | "m" | "cs" | "fs" | "hs"
            | "erl" | "ex" | "clj" | "jl" | "lua" | "pl" | "sh" | "bash" | "ps1" | "vim"
            | "html" | "css" | "scss" => "code",

            "md" | "txt" | "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => "document",

            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "webp" | "ico" | "heic" | "raw" => {
                "image"
            }

            "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" => "audio",

            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "webm" => "video",

            "zip" | "tar" | "gz" | "7z" | "rar" | "iso" => "archive",

            "exe" | "dll" | "msi" | "app" | "deb" | "rpm" | "bin" => "executable",

            _ => "file",
        };

        (icon, category)
    }
}
