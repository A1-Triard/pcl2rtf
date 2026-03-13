use crate::pcl::PclCommand;
use crate::font::{Font, font_char};
use either::{Left, Right};
use iter_identify_first_last::IteratorIdentifyFirstLastExt;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum Style {
    Regular,
    Italic,
    Bold,
    ItalicBold,
}

impl Style {
    fn enable(self, underline: bool, f: &mut Formatter<'_>) -> Result<bool, fmt::Error> {
        match (self, underline) {
            (Style::Regular, false) => Ok(false),
            (Style::Italic, false) => { write!(f, "{{\\i ")?; Ok(true) },
            (Style::Bold, false) => { write!(f, "{{\\b ")?; Ok(true) },
            (Style::ItalicBold, false) => { write!(f, "{{\\i\\b ")?; Ok(true) },
            (Style::Regular, true) => { write!(f, "{{\\ul ")?; Ok(true) },
            (Style::Italic, true) => { write!(f, "{{\\ul\\i ")?; Ok(true) },
            (Style::Bold, true) => { write!(f, "{{\\ul\\b ")?; Ok(true) },
            (Style::ItalicBold, true) => { write!(f, "{{\\ul\\i\\b ")?; Ok(true) },
        }
    }
}

#[derive(Debug)]
struct Span {
    text: String,
    style: Style,
    underline: bool,
}

#[derive(Debug)]
struct Line {
    indent: u32,
    spans: Vec<Span>,
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
                for span in &line.spans {
                    let group = span.style.enable(span.underline, f)?;
                    for c in span.text.chars() {
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
                    if group {
                        write!(f, "}}")?;
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

impl Display for PclToRtfError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            PclToRtfError::UnexpectedEnd => write!(f, "Unexpected end of file"),
            PclToRtfError::UnexpectedCommand(x) => write!(f, "Unexpected command at {x:X}h"),
        }
    }
}

pub fn pcl_to_rtf(pcl: &mut dyn Iterator<Item=(PclCommand, u32)>) -> Result<Rtf, PclToRtfError> {
    enum State { PageStart, LineStart(bool), Text, LineEnd }
    let mut rtf = Rtf { pages: Vec::new() };
    let mut state = State::PageStart;
    let mut font = Font::X9500;
    let mut use_font = false;
    let mut style = Style::Regular;
    let mut underline = false;
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
                    PclCommand::SecondarySymbolSet(9500, b'X') => {
                        font = Font::X9500;
                        style = Style::Regular;
                    },
                    PclCommand::SecondarySymbolSet(9508, b'X') => {
                        font = Font::X9508;
                        style = Style::Regular;
                    },
                    PclCommand::SecondarySymbolSet(9501, b'X') => {
                        font = Font::X9500;
                        style = Style::Italic;
                    },
                    PclCommand::SecondarySymbolSet(9502, b'X') => {
                        font = Font::X9500;
                        style = Style::Bold;
                    },
                    PclCommand::SecondarySymbolSet(9503, b'X') => {
                        font = Font::X9500;
                        style = Style::ItalicBold;
                    },
                    PclCommand::EnableUnderline => underline = true,
                    PclCommand::DisableUnderline => underline = false,
                    PclCommand::VerticalCursorPositioning(Left(0)) => { },
                    PclCommand::Char(13) => { },
                    PclCommand::Char(14) => use_font = true,
                    PclCommand::Char(15) => use_font = false,
                    PclCommand::VerticalCursorPositioning(Right(x)) if x >= 0 => {
                        rtf.pages.push(Page {
                            top_margin: u32::try_from(x).unwrap() * 24 / 5,
                            lines: Vec::new(),
                        });
                        state = State::LineStart(true);
                    },
                    _ => return Err(PclToRtfError::UnexpectedCommand(offset)),
                }
            },
            State::LineStart(allow_line_start) => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::SecondarySymbolSet(9500, b'X') => {
                        font = Font::X9500;
                        style = Style::Regular;
                    },
                    PclCommand::SecondarySymbolSet(9508, b'X') => {
                        font = Font::X9508;
                        style = Style::Regular;
                    },
                    PclCommand::SecondarySymbolSet(9501, b'X') => {
                        font = Font::X9500;
                        style = Style::Italic;
                    },
                    PclCommand::SecondarySymbolSet(9502, b'X') => {
                        font = Font::X9500;
                        style = Style::Bold;
                    },
                    PclCommand::SecondarySymbolSet(9503, b'X') => {
                        font = Font::X9500;
                        style = Style::ItalicBold;
                    },
                    PclCommand::EnableUnderline => underline = true,
                    PclCommand::DisableUnderline => underline = false,
                    PclCommand::Char(14) => use_font = true,
                    PclCommand::Char(15) => use_font = false,
                    PclCommand::HorizontalCursorPositioning(Right(x)) if x >= 0 => {
                        if !allow_line_start {
                            return Err(PclToRtfError::UnexpectedCommand(offset));
                        }
                        rtf.pages.last_mut().unwrap().lines.push(Line {
                            indent: u32::try_from(x).unwrap() * 24 / 5,
                            spans: Vec::new(),
                            space_after: 0,
                        });
                        state = State::Text;
                    },
                    PclCommand::Char(12) => {
                        state = State::PageStart;
                    },
                    _ => return Err(PclToRtfError::UnexpectedCommand(offset)),
                }
            },
            State::Text => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::SecondarySymbolSet(9500, b'X') => {
                        font = Font::X9500;
                        style = Style::Regular;
                    },
                    PclCommand::SecondarySymbolSet(9508, b'X') => {
                        font = Font::X9508;
                        style = Style::Regular;
                    },
                    PclCommand::SecondarySymbolSet(9501, b'X') => {
                        font = Font::X9500;
                        style = Style::Italic;
                    },
                    PclCommand::SecondarySymbolSet(9502, b'X') => {
                        font = Font::X9500;
                        style = Style::Bold;
                    },
                    PclCommand::SecondarySymbolSet(9503, b'X') => {
                        font = Font::X9500;
                        style = Style::ItalicBold;
                    },
                    PclCommand::EnableUnderline => underline = true,
                    PclCommand::DisableUnderline => underline = false,
                    PclCommand::Char(14) => use_font = true,
                    PclCommand::Char(15) => use_font = false,
                    PclCommand::Char(c) if c >= b' ' => {
                        let c = font_char(c, if use_font { Some(font) } else { None });
                        let cur_style = rtf.pages.last().unwrap()
                            .lines.last().unwrap()
                            .spans.last().map(|x| (x.style, x.underline));
                        if cur_style != Some((style, underline)) {
                            rtf.pages.last_mut().unwrap().lines.last_mut().unwrap().spans.push(Span {
                                text: String::new(),
                                style,
                                underline,
                            });
                        }
                        rtf.pages.last_mut().unwrap()
                            .lines.last_mut().unwrap()
                            .spans.last_mut().unwrap()
                            .text.push(c);
                    },
                    PclCommand::HorizontalCursorPositioning(Right(x)) if x != 0 && x % 30 == 0 => {
                        let cur_style = rtf.pages.last().unwrap()
                            .lines.last().unwrap()
                            .spans.last().map(|x| (x.style, x.underline));
                        if cur_style != Some((style, underline)) {
                            rtf.pages.last_mut().unwrap().lines.last_mut().unwrap().spans.push(Span {
                                text: String::new(),
                                style,
                                underline,
                            });
                        }
                        for _ in 0 .. x / 30 {
                            rtf.pages.last_mut().unwrap()
                                .lines.last_mut().unwrap()
                                .spans.last_mut().unwrap()
                                .text.push(' ');
                        }
                    },
                    PclCommand::Char(13) => {
                        state = State::LineEnd;
                    },
                    PclCommand::Char(12) => {
                        state = State::PageStart;
                    },
                    _ => return Err(PclToRtfError::UnexpectedCommand(offset)),
                }
            },
            State::LineEnd => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::SecondarySymbolSet(9500, b'X') => {
                        font = Font::X9500;
                        style = Style::Regular;
                    },
                    PclCommand::SecondarySymbolSet(9508, b'X') => {
                        font = Font::X9508;
                        style = Style::Regular;
                    },
                    PclCommand::SecondarySymbolSet(9501, b'X') => {
                        font = Font::X9500;
                        style = Style::Italic;
                    },
                    PclCommand::SecondarySymbolSet(9502, b'X') => {
                        font = Font::X9500;
                        style = Style::Bold;
                    },
                    PclCommand::SecondarySymbolSet(9503, b'X') => {
                        font = Font::X9500;
                        style = Style::ItalicBold;
                    },
                    PclCommand::EnableUnderline => underline = true,
                    PclCommand::DisableUnderline => underline = false,
                    PclCommand::Char(14) => use_font = true,
                    PclCommand::Char(15) => use_font = false,
                    PclCommand::VerticalCursorPositioning(Right(x)) if x >= 48 => { // 48 > 230 * 5 / 24
                        let space_after = u32::try_from(x).unwrap() * 24 / 5 - 230; // 230 == 11.5 * 1440 / 72
                        rtf.pages.last_mut().unwrap().lines.last_mut().unwrap().space_after = space_after;
                        state = State::LineStart(true);
                    },
                    PclCommand::VerticalCursorPositioning(Right(_)) => {
                        state = State::LineStart(false);
                    },
                    _ => return Err(PclToRtfError::UnexpectedCommand(offset)),
                }
            },
        }
    }
}
