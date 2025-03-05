use crossterm::{
    style::{self, SetForegroundColor},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineType {
    Normal,
    CodeBegin,
    CodeInner,
    CodeEnd,
}

pub struct MarkdownRender {
    prev_line_type: LineType,
    code_active: bool,
}

impl MarkdownRender {
    pub fn new() -> Self {
        Self {
            prev_line_type: LineType::Normal,
            code_active: false,
        }
    }

    pub fn render_line_mut(&mut self, line: &str) -> String {
        let (line_type, is_code) = self.check_line(line);
        let output = if is_code {
            format!("{}", SetForegroundColor(style::Color::Yellow)) + line
                + &format!("{}", SetForegroundColor(style::Color::Reset))
        } else {
            line.to_string()
        };
        self.prev_line_type = line_type;
        self.code_active = is_code;
        output
    }

    fn check_line(&self, line: &str) -> (LineType, bool) {
        let mut line_type = self.prev_line_type;
        let mut is_code = self.code_active;

        if line.trim_start().starts_with("```") {
            match line_type {
                LineType::Normal | LineType::CodeEnd => {
                    line_type = LineType::CodeBegin;
                    is_code = false;
                }
                LineType::CodeBegin | LineType::CodeInner => {
                    line_type = LineType::CodeEnd;
                    is_code = false;
                }
            }
        } else {
            match line_type {
                LineType::Normal => {}
                LineType::CodeEnd => {
                    line_type = LineType::Normal;
                    is_code = false;
                }
                LineType::CodeBegin => {
                    line_type = LineType::CodeInner;
                    is_code = true;
                }
                LineType::CodeInner => {
                    is_code = true;
                }
            }
        }
        (line_type, is_code)
    }
}
