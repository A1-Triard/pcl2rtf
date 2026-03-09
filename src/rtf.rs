use crate::pcl::PclCommand;
use crate::ru::ru_char;
use either::{Left, Right};
use iter_identify_first_last::IteratorIdentifyFirstLastExt;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
struct Line {
    indent: u32,
    text: String,
    space_after: u32,
}

#[derive(Debug)]
struct Page {
    top_margin: u32,
    lines: Vec<Line>,
}

#[derive(Debug)]
pub struct Rtf {
    pages: Vec<Page>,
}

impl Display for Rtf {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        writeln!(f, "{{\\rtf1\\deff0")?;
        writeln!(f, "{{\\fonttbl")?;
        writeln!(f, "{{\\f0\\fswiss\\fcharset0 DotMatrix;}}")?;
        writeln!(f, "}}")?;
        writeln!(f, "\\f0\\fs23")?;
        writeln!(f, "\\paperw11906\\paperh16838")?;
        writeln!(f, "\\margl0\\margr0\\margb0")?;
        for (is_first_page, page) in self.pages.iter().identify_first() {
            if !is_first_page {
                writeln!(f, "{{\\sect\\sbkpage}}")?;
            }
            writeln!(f, "\\margt{}", page.top_margin)?;
            for (is_last_line, line) in page.lines.iter().identify_last() {
                write!(f, "{{\\pard")?;
                if !is_last_line {
                    write!(f, "\\sa{}", line.space_after)?;
                }
                write!(f, "\\li{}", line.indent)?;
                write!(f, " ")?;
                for c in line.text.chars() {
                    if c == ' ' {
                        write!(f, "\\~")?;
                    } else if c.is_ascii() {
                        if c == '\\' || c == '{' || c == '}' {
                            write!(f, "\\")?;
                        }
                        write!(f, "{c}")?;
                    } else {
                        write!(f, "\\u{}?", u32::from(c) as i32)?;
                    }
                }
                writeln!(f, "\\par}}")?;
            }
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum PclToRtfError {
    UnexpectedEnd,
    UnexpectedCommand(u32),
}

pub fn pcl_to_rtf(pcl: &mut dyn Iterator<Item=(PclCommand, u32)>) -> Result<Rtf, PclToRtfError> {
    enum State { PageStart, LineStart, Text, LineEnd }
    let mut rtf = Rtf { pages: Vec::new() };
    let mut state = State::PageStart;
    loop {
        match state {
            State::PageStart => {
                let Some((command, offset)) = pcl.next() else { return Ok(rtf); };
                match command {
                    PclCommand::LineTermination(0) => { },
                    PclCommand::ClearHorizontalMargins => { },
                    PclCommand::VerticalMotionIndex(_) => { },
                    PclCommand::RasterGraphicsPresentationMode(_) => { },
                    PclCommand::EndOfLineWrap(_) => { },
                    PclCommand::SecondarySymbolSet(9500, 88) => { },
                    PclCommand::VerticalCursorPositioning(Left(0)) => { },
                    PclCommand::Char(13) => { },
                    PclCommand::Char(14) => { },
                    PclCommand::VerticalCursorPositioning(Right(x)) if x >= 0 => {
                        rtf.pages.push(Page {
                            top_margin: u32::try_from(x).unwrap() * 24 / 5,
                            lines: Vec::new(),
                        });
                        state = State::LineStart;
                    },
                    _ => {
                        eprintln!("{command:?}");
                        return Err(PclToRtfError::UnexpectedCommand(offset));
                    },
                }
            },
            State::LineStart => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::HorizontalCursorPositioning(Right(x)) if x >= 0 => {
                        rtf.pages.last_mut().unwrap().lines.push(Line {
                            indent: u32::try_from(x).unwrap() * 24 / 5,
                            text: String::new(),
                            space_after: 0,
                        });
                        state = State::Text;
                    },
                    _ => return Err(PclToRtfError::UnexpectedCommand(offset)),
                }
            },
            State::Text => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::Char(c) if c >= b' ' => {
                        let c = ru_char(c);
                        rtf.pages.last_mut().unwrap().lines.last_mut().unwrap().text.push(c);
                    },
                    PclCommand::HorizontalCursorPositioning(Right(x)) if x != 0 && x % 30 == 0 => {
                        for _ in 0 .. x / 30 {
                            rtf.pages.last_mut().unwrap().lines.last_mut().unwrap().text.push(' ');
                        }
                    },
                    PclCommand::Char(13) => {
                        state = State::LineEnd;
                    },
                    PclCommand::Char(12) => {
                        state = State::PageStart;
                    },
                    _ => {
                        eprintln!("{command:?}");
                        return Err(PclToRtfError::UnexpectedCommand(offset));
                    },
                }
            },
            State::LineEnd => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::VerticalCursorPositioning(Right(x)) if x >= 60 => {
                        let space_after = (u32::try_from(x).unwrap() - 60) * 24 / 5;
                        rtf.pages.last_mut().unwrap().lines.last_mut().unwrap().space_after = space_after;
                        state = State::LineStart;
                    },
                    _ => {
                        eprintln!("{command:?}");
                        return Err(PclToRtfError::UnexpectedCommand(offset));
                    },
                }
            },
        }
    }
}
