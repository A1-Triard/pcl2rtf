use crate::pcl::PclCommand;
use crate::ru::ru_char;
use either::{Left, Right};

#[derive(Debug)]
pub struct Rtf {
    lines: Vec<String>,
    left_margin: u32,
    top_margin: u32,
    sl: Option<u16>,
}

#[derive(Debug)]
pub enum PclToRtfError {
    UnexpectedEnd,
    TextInPreamble(u32),
    UnexpectedCommand(u32),
}

pub fn pcl_to_rtf(pcl: &mut dyn Iterator<Item=(PclCommand, u32)>) -> Result<Rtf, PclToRtfError> {
    enum State { Preamble, LeftMarginSet, TopMarginSet, Text(bool), NewLine, NewLineMargin, PageEnd, End }
    let mut rtf = Rtf { lines: Vec::new(), left_margin: 0, top_margin: 0, sl: None };
    let mut state = State::Preamble;
    loop {
        match state {
            State::Preamble => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::LineTermination(0) => { },
                    PclCommand::ClearHorizontalMargins => { },
                    PclCommand::VerticalMotionIndex(_) => { },
                    PclCommand::RasterGraphicsPresentationMode(_) => { },
                    PclCommand::EndOfLineWrap(_) => { },
                    PclCommand::SecondarySymbolSet(8300, 88) => { },
                    PclCommand::VerticalCursorPositioning(Left(0)) => { },
                    PclCommand::Char(c) if c >= b' ' => return Err(PclToRtfError::TextInPreamble(offset)),
                    PclCommand::Char(13) => { },
                    PclCommand::Char(14) => { },
                    PclCommand::VerticalCursorPositioning(Right(x)) if x >= 0 => {
                        rtf.top_margin = u32::try_from(x).unwrap() * 24 / 5;
                        state = State::TopMarginSet;
                    },
                    PclCommand::HorizontalCursorPositioning(Right(x)) if x >= 0 => {
                        rtf.left_margin = u32::try_from(x).unwrap() * 24 / 5;
                        state = State::LeftMarginSet;
                    },
                    _ => return Err(PclToRtfError::UnexpectedCommand(offset)),
                }
            },
            State::LeftMarginSet => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::VerticalCursorPositioning(Right(x)) if x >= 0 => {
                        rtf.top_margin = u32::try_from(x).unwrap() * 24 / 5;
                        state = State::Text(true);
                    },
                    _ => return Err(PclToRtfError::UnexpectedCommand(offset)),
                }
            },
            State::TopMarginSet => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::HorizontalCursorPositioning(Right(x)) if x >= 0 => {
                        rtf.left_margin = u32::try_from(x).unwrap() * 24 / 5;
                        state = State::Text(true);
                    },
                    _ => return Err(PclToRtfError::UnexpectedCommand(offset)),
                }
            },
            State::Text(new_line) => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::Char(c) if c >= b' ' => {
                        let c = ru_char(c);
                        if new_line {
                            rtf.lines.push(String::new());
                        }
                        rtf.lines.last_mut().unwrap().push(c);
                        state = State::Text(false);
                    },
                    PclCommand::HorizontalCursorPositioning(Right(x)) if x != 0 && x % 45 == 0 => {
                        if new_line {
                            rtf.lines.push(String::new());
                        }
                        for _ in 0 .. x / 45 {
                            rtf.lines.last_mut().unwrap().push(' ');
                        }
                        state = State::Text(false);
                    },
                    PclCommand::Char(13) => {
                        state = State::NewLine;
                    },
                    _ => {
                        eprintln!("{command:?}");
                        return Err(PclToRtfError::UnexpectedCommand(offset));
                    },
                }
            },
            State::NewLine => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::VerticalCursorPositioning(Right(75)) => {
                        if let Some(sl) = rtf.sl {
                            if sl != 360 {
                                return Err(PclToRtfError::UnexpectedCommand(offset));
                            }
                        }
                        rtf.sl = Some(360);
                        state = State::NewLineMargin;
                    },
                    PclCommand::VerticalCursorPositioning(Right(x)) => {
                        state = State::PageEnd;
                    },
                    _ => {
                        eprintln!("{command:?}");
                        return Err(PclToRtfError::UnexpectedCommand(offset));
                    },
                }
            },
            State::NewLineMargin => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::HorizontalCursorPositioning(Right(x)) if x >= 0 => {
                        if rtf.left_margin != u32::try_from(x).unwrap() * 24 / 5 {
                            return Err(PclToRtfError::UnexpectedCommand(offset));
                        }
                        state = State::Text(true);
                    },
                    _ => {
                        eprintln!("{command:?}");
                        return Err(PclToRtfError::UnexpectedCommand(offset));
                    },
                }
            },
            State::PageEnd => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::Char(12) => {
                        state = State::End; // TODO new page
                    },
                    _ => {
                        eprintln!("{command:?}");
                        return Err(PclToRtfError::UnexpectedCommand(offset));
                    },
                }
            },
            State::End => {
                let Some((command, offset)) = pcl.next() else { return Ok(rtf); };
                match command {
                    PclCommand::Char(15) => { },
                    _ => return Err(PclToRtfError::UnexpectedCommand(offset)),
                }
            },
        }
    }
}
